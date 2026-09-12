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

export function getCoords(
  e: React.MouseEvent,
  container: HTMLDivElement,
  img: HTMLImageElement,
): Coords | null {
  const cRect = container.getBoundingClientRect();
  const iRect = img.getBoundingClientRect();
  const naturalW = img.naturalWidth;
  const naturalH = img.naturalHeight;
  const elemW = iRect.width;
  const elemH = iRect.height;
  const ratio = Math.min(elemW / naturalW, elemH / naturalH);
  const displayW = naturalW * ratio;
  const displayH = naturalH * ratio;
  const objectFitOffsetX = (elemW - displayW) / 2;
  const objectFitOffsetY = (elemH - displayH) / 2;
  const imgElemOffsetX = iRect.left - cRect.left - BORDER;
  const imgElemOffsetY = iRect.top - cRect.top - BORDER;
  const totalOffsetX = imgElemOffsetX + objectFitOffsetX;
  const totalOffsetY = imgElemOffsetY + objectFitOffsetY;
  const overlayX = e.clientX - cRect.left - BORDER;
  const overlayY = e.clientY - cRect.top - BORDER;
  const imgX = overlayX - totalOffsetX;
  const imgY = overlayY - totalOffsetY;
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
