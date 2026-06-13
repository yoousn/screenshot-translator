from __future__ import annotations

import math
from dataclasses import dataclass

import cv2
import numpy as np

from .contract import PPOCRV6_BLANK_INDEX, PPOCRV6_CLASS_COUNT, PPOCRv6ModelContract


@dataclass(frozen=True)
class RecognitionResult:
    text: str
    confidence: float


def _probabilities(output: np.ndarray) -> np.ndarray:
    row_sums = output.sum(axis=-1)
    if output.min() >= 0 and output.max() <= 1.0 and np.allclose(row_sums, 1.0, atol=1e-3):
        return output
    stable = output - output.max(axis=-1, keepdims=True)
    exp = np.exp(stable)
    return exp / exp.sum(axis=-1, keepdims=True)


def decode_ctc(output: np.ndarray, contract: PPOCRv6ModelContract) -> list[RecognitionResult]:
    if output.ndim != 3 or output.shape[-1] != PPOCRV6_CLASS_COUNT:
        raise RuntimeError(f"unexpected PP-OCRv6 recognition output shape: {output.shape}")
    probs = _probabilities(output)
    indices = probs.argmax(axis=-1)
    scores = probs.max(axis=-1)
    results: list[RecognitionResult] = []
    for batch_indices, batch_scores in zip(indices, scores):
        text_parts: list[str] = []
        accepted_scores: list[float] = []
        previous = -1
        for class_index, score in zip(batch_indices.tolist(), batch_scores.tolist()):
            if class_index != previous and class_index != PPOCRV6_BLANK_INDEX:
                text_parts.append(contract.character_for_class(int(class_index)))
                accepted_scores.append(float(score))
            previous = int(class_index)
        results.append(
            RecognitionResult(
                text="".join(text_parts),
                confidence=float(np.mean(accepted_scores)) if accepted_scores else 0.0,
            )
        )
    return results


class PPOCRv6Recognizer:
    def __init__(self, session, contract: PPOCRv6ModelContract) -> None:
        self.session = session
        self.contract = contract
        self.input_name = session.get_inputs()[0].name

    def __call__(self, crops_bgr: list[np.ndarray]) -> list[RecognitionResult]:
        if not crops_bgr:
            return []
        ratios = [crop.shape[1] / max(1.0, float(crop.shape[0])) for crop in crops_bgr]
        target_width = max(320, int(math.ceil(max(ratios) * 48 / 32) * 32))
        target_width = min(target_width, 3200)
        batch = np.stack([self._preprocess(crop, target_width) for crop in crops_bgr]).astype(np.float32)
        output = self.session.run(None, {self.input_name: batch})[0]
        return decode_ctc(output, self.contract)

    @staticmethod
    def _preprocess(image_bgr: np.ndarray, target_width: int) -> np.ndarray:
        image_rgb = cv2.cvtColor(image_bgr, cv2.COLOR_BGR2RGB)
        height, width = image_rgb.shape[:2]
        resized_width = min(target_width, max(1, int(math.ceil(48 * width / max(1.0, float(height))))))
        resized = cv2.resize(image_rgb, (resized_width, 48))
        normalized = resized.astype(np.float32).transpose((2, 0, 1)) / 255.0
        normalized = (normalized - 0.5) / 0.5
        padded = np.zeros((3, 48, target_width), dtype=np.float32)
        padded[:, :, :resized_width] = normalized
        return padded
