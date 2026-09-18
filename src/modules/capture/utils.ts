const BORDER = 1;

export interface Coords {
  overlayX: number;
  overlayY: number;
  imgX: number;
  imgY: number;
  displayW: number;
  displayH: number;
  naturalW: number;
  naturalH: number;
}

export interface ImageGeometry {
  offsetX: number;
  offsetY: number;
  displayW: number;
  displayH: number;
  naturalW: number;
  naturalH: number;
}

export function getImageGeometry(
  container: HTMLDivElement,
  img: HTMLImageElement,
): ImageGeometry | null {
  const cRect = container.getBoundingClientRect();
  const iRect = img.getBoundingClientRect();
  const naturalW = img.naturalWidth;
  const naturalH = img.naturalHeight;
  if (naturalW <= 0 || naturalH <= 0 || iRect.width <= 0 || iRect.height <= 0) return null;
  const ratio = Math.min(iRect.width / naturalW, iRect.height / naturalH);
  const displayW = naturalW * ratio;
  const displayH = naturalH * ratio;
  const objectFitOffsetX = (iRect.width - displayW) / 2;
  const objectFitOffsetY = (iRect.height - displayH) / 2;
  return {
    offsetX: iRect.left - cRect.left - BORDER + objectFitOffsetX,
    offsetY: iRect.top - cRect.top - BORDER + objectFitOffsetY,
    displayW,
    displayH,
    naturalW,
    naturalH,
  };
}

export function getCoords(
  e: { clientX: number; clientY: number },
  container: HTMLDivElement,
  img: HTMLImageElement,
): Coords | null {
  const cRect = container.getBoundingClientRect();
  const geometry = getImageGeometry(container, img);
  if (!geometry) return null;
  const { offsetX, offsetY, displayW, displayH, naturalW, naturalH } = geometry;
  const overlayX = e.clientX - cRect.left - BORDER;
  const overlayY = e.clientY - cRect.top - BORDER;
  const imgX = overlayX - offsetX;
  const imgY = overlayY - offsetY;
  if (imgX < 0 || imgY < 0 || imgX > displayW || imgY > displayH) return null;
  return { overlayX, overlayY, imgX, imgY, displayW, displayH, naturalW, naturalH };
}

export function imgToNatural(imgX: number, imgY: number, c: Coords) {
  return {
    x: Math.round((imgX / c.displayW) * c.naturalW),
    y: Math.round((imgY / c.displayH) * c.naturalH),
  };
}

export function getTimestamp(): string {
  const now = new Date();
  const pad = (n: number) => String(n).padStart(2, "0");
  return `${now.getFullYear()}-${pad(now.getMonth() + 1)}-${pad(now.getDate())}_${pad(now.getHours())}-${pad(now.getMinutes())}-${pad(now.getSeconds())}`;
}
