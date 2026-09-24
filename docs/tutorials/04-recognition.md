# Chapter 4: Recognize visual states

Recognition answers whether the current captured frame matches a node. Choose
the simplest recognizer that distinguishes the state reliably.

## 4.1 DirectHit

`DirectHit` always matches. Use it for a deliberate transition that requires no
visual condition:

```json
"recognition": { "type": "DirectHit" }
```

Do not use it to hide an unreliable visual check.

## 4.2 OCR

OCR matches visible text:

```json
"recognition": {
  "type": "OCR",
  "expected": "Invalid account or password",
  "threshold": 0.6,
  "roi": [160, 380, 190, 40]
}
```

An ROI is `[x, y, width, height]` in capture-frame coordinates. Keep enough
space for wrapped text. WatchDog can group nearby OCR words across lines, but
an ROI that cuts off a word cannot recover it.

Use the Node Test view to inspect detected text, candidates, confidence, and
candidate selection before using OCR in a full run.

## 4.3 TemplateMatch

Template matching finds a reference image:

```json
"recognition": {
  "type": "TemplateMatch",
  "template": "sample/notification.png",
  "threshold": 0.8
}
```

Place user templates under the images folder shown in Settings. Crop templates
tightly and capture them at the same scale as the target UI.

## 4.4 ColorMatch

Color matching finds pixels inside a configured color range:

```json
"recognition": {
  "type": "ColorMatch",
  "lower": [90, 80, 70],
  "upper": [130, 255, 255]
}
```

Use it for stable status colors or simple indicators. Avoid broad ranges that
also match the page background.

## 4.5 Candidate selection and inversion

Recognizers can produce several candidates. Ordering and candidate index select
which result becomes the effective match. Inversion reverses a valid match
condition, but it does not convert configuration or candidate-selection errors
into success.

## 4.6 Recognition checklist

1. Select the same capture target and capture area the pipeline will use.
2. Test the recognizer in isolation.
3. Inspect every candidate, not only the selected one.
4. Narrow the ROI when unrelated content matches.
5. Set the threshold from observed scores with some margin.
6. Test both the matching and non-matching states.

Next: [Execute actions and control timing](05-actions-and-timing.md).
