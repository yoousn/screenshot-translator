from __future__ import annotations

import time
from pathlib import Path

import cv2
import numpy as np

from .contract import create_session, load_model_contract, validate_fixed_input_probe
from .detector import PPOCRv6Detector
from .recognizer import PPOCRv6Recognizer


def _load_image(path: Path) -> np.ndarray:
    data = np.fromfile(path, dtype=np.uint8)
    image = cv2.imdecode(data, cv2.IMREAD_COLOR)
    if image is None:
        raise RuntimeError(f"failed to decode image: {path}")
    return image


def _crop_text_region(image: np.ndarray, points: np.ndarray) -> np.ndarray:
    crop_width = max(
        1,
        int(max(np.linalg.norm(points[0] - points[1]), np.linalg.norm(points[2] - points[3]))),
    )
    crop_height = max(
        1,
        int(max(np.linalg.norm(points[0] - points[3]), np.linalg.norm(points[1] - points[2]))),
    )
    target = np.asarray(
        [[0, 0], [crop_width, 0], [crop_width, crop_height], [0, crop_height]],
        dtype=np.float32,
    )
    matrix = cv2.getPerspectiveTransform(points.astype(np.float32), target)
    crop = cv2.warpPerspective(
        image,
        matrix,
        (crop_width, crop_height),
        flags=cv2.INTER_CUBIC,
        borderMode=cv2.BORDER_REPLICATE,
    )
    if crop.shape[0] / max(1.0, float(crop.shape[1])) >= 1.5:
        crop = np.rot90(crop)
    return crop


class PPOCRv6Adapter:
    def __init__(self, model_root: Path) -> None:
        started = time.perf_counter()
        self.contract = load_model_contract(model_root)
        self.det_session = create_session(self.contract.det_model_path)
        self.rec_session = create_session(self.contract.rec_model_path)
        self.contract_probe = validate_fixed_input_probe(self.contract, self.det_session, self.rec_session)
        self.detector = PPOCRv6Detector(self.det_session, self.contract.det_config)
        self.recognizer = PPOCRv6Recognizer(self.rec_session, self.contract)
        self.init_ms = int(round((time.perf_counter() - started) * 1000))

    def probe(self) -> dict:
        return {
            "status": "success",
            "engine": "ppocr-v6-onnxruntime",
            "modelVersion": "v6",
            "probe": "fixed-input-contract",
            "contract": self.contract_probe,
            "timings": {"init_ms": self.init_ms},
        }

    def run(self, image_path: Path) -> dict:
        total_started = time.perf_counter()
        image = _load_image(image_path)

        det_started = time.perf_counter()
        detection = self.detector(image)
        det_ms = int(round((time.perf_counter() - det_started) * 1000))

        crops = [_crop_text_region(image, box) for box in detection.boxes]
        rec_started = time.perf_counter()
        recognition = self.recognizer(crops)
        rec_ms = int(round((time.perf_counter() - rec_started) * 1000))

        blocks = []
        for box, det_score, rec_result in zip(detection.boxes, detection.scores, recognition):
            text = rec_result.text.strip()
            if not text:
                continue
            blocks.append(
                {
                    "text": text,
                    "confidence": rec_result.confidence,
                    "detection_confidence": float(det_score),
                    "box_coords": [[int(round(float(x))), int(round(float(y)))] for x, y in box],
                }
            )

        return {
            "status": "success",
            "engine": "ppocr-v6-onnxruntime",
            "modelVersion": "v6",
            "selectedLang": "v6",
            "selectedVariant": "original",
            "blocks": blocks,
            "timings": {
                "total_ms": int(round((time.perf_counter() - total_started) * 1000)),
                "det_ms": det_ms,
                "rec_ms": rec_ms,
                "selected_init_ms": self.init_ms,
                "candidate_count": 1,
            },
            "candidates": [
                {
                    "lang": "v6",
                    "variant": "original",
                    "blocks": len(blocks),
                    "timings": {"det_ms": det_ms, "rec_ms": rec_ms},
                }
            ],
        }
