export type OcrModelPackOperationPhase =
  | "queued"
  | "resolving-index"
  | "downloading"
  | "verifying"
  | "installing"
  | "self-testing"
  | "completed"
  | "failed";

export type OcrModelPackOperation = {
  operationId: string;
  packId: string;
  phase: OcrModelPackOperationPhase;
  percent: number;
  recoverable: boolean;
  message: string;
  nextAction?: string;
};

export const OCR_MODEL_PACK_PROGRESS_EVENT = "ysn-ocr-model-pack-progress";

export const getOperationPhaseLabel = (phase: OcrModelPackOperationPhase) => {
  switch (phase) {
    case "queued": return "Queued";
    case "resolving-index": return "Resolving index";
    case "downloading": return "Downloading";
    case "verifying": return "Verifying";
    case "installing": return "Installing";
    case "self-testing": return "Self-testing";
    case "completed": return "Completed";
    case "failed": return "Failed";
    default: return phase;
  }
};
