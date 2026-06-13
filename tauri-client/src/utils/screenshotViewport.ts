import type { ScreenshotPhysicalBounds } from "../types/screenshot";

export const getViewportDevicePixelRatio = () => {
  const ratio = window.devicePixelRatio || 1;
  return Number.isFinite(ratio) && ratio > 0 ? ratio : 1;
};

export const getLogicalCanvasSize = (physicalBounds?: ScreenshotPhysicalBounds | null) => {
  if (physicalBounds && physicalBounds.width > 0 && physicalBounds.height > 0) {
    const ratio = getViewportDevicePixelRatio();
    return {
      width: Math.max(1, Math.round(physicalBounds.width / ratio)),
      height: Math.max(1, Math.round(physicalBounds.height / ratio)),
    };
  }
  return {
    width: Math.max(1, window.innerWidth),
    height: Math.max(1, window.innerHeight),
  };
};
