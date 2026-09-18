import type { Recognition } from "../../../api/pipeline";
import type { OverlayRect } from "../stores/useOverlayStore";

export function resolveRecognitionOverlayRoi(
  recognition: Recognition,
  imageDimensions: { width: number; height: number } | null,
): OverlayRect | null {
  const source = recognition.roi ?? [0, 0, 0, 0];
  // A node reference is resolved only by the backend against a prior result.
  if (typeof source === "string") return null;
  const offset = recognition.roi_offset ?? [0, 0, 0, 0];
  const fullFrame = source.every((value) => value === 0);
  if (fullFrame && !imageDimensions) return null;

  const requested = fullFrame
    ? { x: 0, y: 0, w: imageDimensions!.width, h: imageDimensions!.height }
    : { x: source[0], y: source[1], w: source[2], h: source[3] };
  const adjusted = {
    x: requested.x + offset[0],
    y: requested.y + offset[1],
    w: requested.w + offset[2],
    h: requested.h + offset[3],
  };
  if (adjusted.w <= 0 || adjusted.h <= 0) return null;
  if (!imageDimensions) return adjusted;

  const left = Math.max(0, adjusted.x);
  const top = Math.max(0, adjusted.y);
  const right = Math.min(imageDimensions.width, adjusted.x + adjusted.w);
  const bottom = Math.min(imageDimensions.height, adjusted.y + adjusted.h);
  if (left >= right || top >= bottom) return null;
  return { x: left, y: top, w: right - left, h: bottom - top };
}
