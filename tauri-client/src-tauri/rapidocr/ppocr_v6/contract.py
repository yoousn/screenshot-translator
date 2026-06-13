from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path
from typing import Any

import numpy as np
import onnxruntime as ort
import yaml


PPOCRV6_DICTIONARY_SIZE = 18_708
PPOCRV6_CLASS_COUNT = 18_710
PPOCRV6_BLANK_INDEX = 0
PPOCRV6_SPACE_INDEX = 18_709
PPOCRV6_MODEL_FILES = {
    "det_model": "PP-Ocrv6_small_det.onnx",
    "det_config": "PP-Ocrv6_small_det.yml",
    "rec_model": "OCRv6_small_rec.onnx",
    "rec_config": "PP-OCRv6_small_rec.yml",
}


class PPOCRv6ContractError(RuntimeError):
    pass


@dataclass(frozen=True)
class PPOCRv6ModelContract:
    model_root: Path
    det_model_path: Path
    rec_model_path: Path
    characters: tuple[str, ...]
    det_config: dict[str, Any]
    rec_config: dict[str, Any]

    def character_for_class(self, class_index: int) -> str:
        if class_index == PPOCRV6_BLANK_INDEX:
            return ""
        if 1 <= class_index <= PPOCRV6_DICTIONARY_SIZE:
            return self.characters[class_index - 1]
        if class_index == PPOCRV6_SPACE_INDEX:
            return " "
        raise PPOCRv6ContractError(
            f"PP-OCRv6 emitted unsupported class index {class_index}; "
            f"expected 0..{PPOCRV6_SPACE_INDEX}"
        )


def _load_yaml(path: Path) -> dict[str, Any]:
    try:
        payload = yaml.safe_load(path.read_text(encoding="utf-8"))
    except Exception as exc:
        raise PPOCRv6ContractError(f"failed to read PP-OCRv6 config: {path}") from exc
    if not isinstance(payload, dict):
        raise PPOCRv6ContractError(f"invalid PP-OCRv6 config object: {path}")
    return payload


def load_model_contract(model_root: Path) -> PPOCRv6ModelContract:
    root = model_root.resolve()
    paths = {name: root / filename for name, filename in PPOCRV6_MODEL_FILES.items()}
    missing = [str(path) for path in paths.values() if not path.is_file()]
    if missing:
        raise PPOCRv6ContractError("missing PP-OCRv6 model files: " + ", ".join(missing))

    det_config = _load_yaml(paths["det_config"])
    rec_config = _load_yaml(paths["rec_config"])
    characters = (
        rec_config.get("PostProcess", {}).get("character_dict")
        if isinstance(rec_config.get("PostProcess"), dict)
        else None
    )
    if not isinstance(characters, list) or not all(isinstance(item, str) for item in characters):
        raise PPOCRv6ContractError("PP-OCRv6 recognition config has no valid character_dict")
    if len(characters) != PPOCRV6_DICTIONARY_SIZE:
        raise PPOCRv6ContractError(
            f"PP-OCRv6 dictionary contract failed: expected {PPOCRV6_DICTIONARY_SIZE}, "
            f"got {len(characters)}"
        )

    contract = PPOCRv6ModelContract(
        model_root=root,
        det_model_path=paths["det_model"],
        rec_model_path=paths["rec_model"],
        characters=tuple(characters),
        det_config=det_config,
        rec_config=rec_config,
    )
    if contract.character_for_class(1) != characters[0]:
        raise PPOCRv6ContractError("PP-OCRv6 class 1 must map to dictionary[0]")
    if contract.character_for_class(PPOCRV6_DICTIONARY_SIZE) != characters[-1]:
        raise PPOCRv6ContractError("PP-OCRv6 dictionary tail mapping is invalid")
    if contract.character_for_class(PPOCRV6_SPACE_INDEX) != " ":
        raise PPOCRv6ContractError("PP-OCRv6 class 18709 must map to the single implicit space")
    return contract


def create_session(model_path: Path) -> ort.InferenceSession:
    options = ort.SessionOptions()
    options.log_severity_level = 3
    options.graph_optimization_level = ort.GraphOptimizationLevel.ORT_ENABLE_ALL
    return ort.InferenceSession(
        str(model_path),
        sess_options=options,
        providers=["CPUExecutionProvider"],
    )


def validate_fixed_input_probe(
    contract: PPOCRv6ModelContract,
    det_session: ort.InferenceSession,
    rec_session: ort.InferenceSession,
) -> dict[str, Any]:
    rec_inputs = rec_session.get_inputs()
    if len(rec_inputs) != 1:
        raise PPOCRv6ContractError(f"PP-OCRv6 recognition model expected one input, got {len(rec_inputs)}")
    rec_probe = np.zeros((1, 3, 48, 320), dtype=np.float32)
    rec_outputs = rec_session.run(None, {rec_inputs[0].name: rec_probe})
    if len(rec_outputs) != 1 or rec_outputs[0].ndim != 3:
        shape = getattr(rec_outputs[0], "shape", None) if rec_outputs else None
        raise PPOCRv6ContractError(f"PP-OCRv6 recognition probe returned invalid output shape: {shape}")
    rec_shape = tuple(int(value) for value in rec_outputs[0].shape)
    if rec_shape[-1] != PPOCRV6_CLASS_COUNT:
        raise PPOCRv6ContractError(
            f"PP-OCRv6 class contract failed: expected {PPOCRV6_CLASS_COUNT}, got {rec_shape[-1]}"
        )

    det_inputs = det_session.get_inputs()
    if len(det_inputs) != 1:
        raise PPOCRv6ContractError(f"PP-OCRv6 detection model expected one input, got {len(det_inputs)}")
    det_probe = np.zeros((1, 3, 32, 32), dtype=np.float32)
    det_outputs = det_session.run(None, {det_inputs[0].name: det_probe})
    if len(det_outputs) != 1 or det_outputs[0].ndim != 4 or det_outputs[0].shape[1] != 1:
        shape = getattr(det_outputs[0], "shape", None) if det_outputs else None
        raise PPOCRv6ContractError(f"PP-OCRv6 detection probe returned invalid output shape: {shape}")

    return {
        "dictionarySize": len(contract.characters),
        "classCount": rec_shape[-1],
        "blankIndex": PPOCRV6_BLANK_INDEX,
        "spaceIndex": PPOCRV6_SPACE_INDEX,
        "spaceValue": contract.character_for_class(PPOCRV6_SPACE_INDEX),
        "recProbeShape": list(rec_shape),
        "detProbeShape": [int(value) for value in det_outputs[0].shape],
    }
