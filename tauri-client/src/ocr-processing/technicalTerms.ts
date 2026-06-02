export const protectedTechnicalTerms = [
  "Windows",
  "PATH",
  "OCR",
  "ONNX",
  "YSN",
  "FFmpeg",
  "ffmpeg.exe",
];

export const restoreProtectedTechnicalTerms = (text: string) => {
  let next = text;
  for (const token of protectedTechnicalTerms) {
    const escapedToken = token.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
    const flags = token === "PATH" ? "g" : "gi";
    next = next.replace(new RegExp(escapedToken, flags), token);
  }
  return next;
};
