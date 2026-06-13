from __future__ import annotations

from dataclasses import dataclass

import cv2
import numpy as np
import pyclipper
from shapely.geometry import Polygon


@dataclass(frozen=True)
class DetectionResult:
    boxes: np.ndarray
    scores: list[float]


class DBPostProcessor:
    def __init__(
        self,
        *,
        thresh: float,
        box_thresh: float,
        max_candidates: int,
        unclip_ratio: float,
    ) -> None:
        self.thresh = thresh
        self.box_thresh = box_thresh
        self.max_candidates = max_candidates
        self.unclip_ratio = unclip_ratio
        self.min_size = 3

    def __call__(self, prediction: np.ndarray, original_shape: tuple[int, int]) -> DetectionResult:
        if prediction.ndim != 4 or prediction.shape[0] != 1 or prediction.shape[1] != 1:
            raise RuntimeError(f"unexpected PP-OCRv6 detection output shape: {prediction.shape}")
        probability_map = prediction[0, 0]
        bitmap = probability_map > self.thresh
        boxes, scores = self._boxes_from_bitmap(
            probability_map,
            bitmap,
            original_shape[1],
            original_shape[0],
        )
        return self._filter_boxes(boxes, scores, original_shape[0], original_shape[1])

    def _boxes_from_bitmap(
        self,
        probability_map: np.ndarray,
        bitmap: np.ndarray,
        dest_width: int,
        dest_height: int,
    ) -> tuple[list[np.ndarray], list[float]]:
        contours, _ = cv2.findContours(
            (bitmap.astype(np.uint8) * 255),
            cv2.RETR_LIST,
            cv2.CHAIN_APPROX_SIMPLE,
        )
        height, width = bitmap.shape
        boxes: list[np.ndarray] = []
        scores: list[float] = []
        for contour in contours[: self.max_candidates]:
            points, short_side = self._mini_box(contour)
            if short_side < self.min_size:
                continue
            score = self._box_score(probability_map, points)
            if score < self.box_thresh:
                continue
            expanded = self._unclip(points)
            if expanded is None:
                continue
            box, short_side = self._mini_box(expanded)
            if short_side < self.min_size + 2:
                continue
            box[:, 0] = np.clip(np.round(box[:, 0] / width * dest_width), 0, dest_width - 1)
            box[:, 1] = np.clip(np.round(box[:, 1] / height * dest_height), 0, dest_height - 1)
            boxes.append(box.astype(np.float32))
            scores.append(float(score))
        return boxes, scores

    @staticmethod
    def _mini_box(contour: np.ndarray) -> tuple[np.ndarray, float]:
        bounding_box = cv2.minAreaRect(contour)
        points = sorted(list(cv2.boxPoints(bounding_box)), key=lambda point: point[0])
        left = sorted(points[:2], key=lambda point: point[1])
        right = sorted(points[2:], key=lambda point: point[1])
        box = np.array([left[0], right[0], right[1], left[1]], dtype=np.float32)
        return box, min(bounding_box[1])

    @staticmethod
    def _box_score(probability_map: np.ndarray, source_box: np.ndarray) -> float:
        height, width = probability_map.shape
        box = source_box.copy()
        xmin = int(np.clip(np.floor(box[:, 0].min()), 0, width - 1))
        xmax = int(np.clip(np.ceil(box[:, 0].max()), 0, width - 1))
        ymin = int(np.clip(np.floor(box[:, 1].min()), 0, height - 1))
        ymax = int(np.clip(np.ceil(box[:, 1].max()), 0, height - 1))
        mask = np.zeros((ymax - ymin + 1, xmax - xmin + 1), dtype=np.uint8)
        box[:, 0] -= xmin
        box[:, 1] -= ymin
        cv2.fillPoly(mask, box.reshape(1, -1, 2).astype(np.int32), 1)
        return float(cv2.mean(probability_map[ymin : ymax + 1, xmin : xmax + 1], mask)[0])

    def _unclip(self, box: np.ndarray) -> np.ndarray | None:
        polygon = Polygon(box)
        if polygon.length <= 0 or polygon.area <= 0:
            return None
        offset = pyclipper.PyclipperOffset()
        offset.AddPath(box.tolist(), pyclipper.JT_ROUND, pyclipper.ET_CLOSEDPOLYGON)
        expanded = offset.Execute(polygon.area * self.unclip_ratio / polygon.length)
        if not expanded:
            return None
        largest = max(expanded, key=lambda points: abs(cv2.contourArea(np.asarray(points, dtype=np.float32))))
        return np.asarray(largest, dtype=np.float32).reshape((-1, 1, 2))

    @staticmethod
    def _filter_boxes(
        boxes: list[np.ndarray],
        scores: list[float],
        image_height: int,
        image_width: int,
    ) -> DetectionResult:
        filtered: list[tuple[np.ndarray, float]] = []
        for box, score in zip(boxes, scores):
            box[:, 0] = np.clip(box[:, 0], 0, image_width - 1)
            box[:, 1] = np.clip(box[:, 1], 0, image_height - 1)
            width = int(np.linalg.norm(box[0] - box[1]))
            height = int(np.linalg.norm(box[0] - box[3]))
            if width > 3 and height > 3:
                filtered.append((box, score))
        filtered.sort(key=lambda item: (round(float(item[0][0][1]) / 10), float(item[0][0][0])))
        if not filtered:
            return DetectionResult(np.empty((0, 4, 2), dtype=np.float32), [])
        return DetectionResult(
            np.asarray([item[0] for item in filtered], dtype=np.float32),
            [item[1] for item in filtered],
        )


class PPOCRv6Detector:
    def __init__(self, session, config: dict) -> None:
        postprocess = config.get("PostProcess", {})
        self.session = session
        self.input_name = session.get_inputs()[0].name
        self.postprocess = DBPostProcessor(
            thresh=float(postprocess.get("thresh", 0.2)),
            box_thresh=float(postprocess.get("box_thresh", 0.45)),
            max_candidates=int(postprocess.get("max_candidates", 3000)),
            unclip_ratio=float(postprocess.get("unclip_ratio", 1.4)),
        )
        self.mean = np.asarray([0.485, 0.456, 0.406], dtype=np.float32)
        self.std = np.asarray([0.229, 0.224, 0.225], dtype=np.float32)

    def __call__(self, image_bgr: np.ndarray) -> DetectionResult:
        original_shape = image_bgr.shape[:2]
        resized = self._resize(image_bgr)
        normalized = (resized.astype(np.float32) / 255.0 - self.mean) / self.std
        tensor = normalized.transpose((2, 0, 1))[np.newaxis, :].astype(np.float32)
        prediction = self.session.run(None, {self.input_name: tensor})[0]
        return self.postprocess(prediction, original_shape)

    @staticmethod
    def _resize(image: np.ndarray, limit_side_len: int = 960) -> np.ndarray:
        height, width = image.shape[:2]
        ratio = min(1.0, float(limit_side_len) / max(height, width))
        resize_height = max(32, int(round(height * ratio / 32) * 32))
        resize_width = max(32, int(round(width * ratio / 32) * 32))
        return cv2.resize(image, (resize_width, resize_height))
