# Chapter 9: Test and debug pipelines

Test the smallest uncertain part before running the complete graph. WatchDog
provides isolated recognition and action tools plus full-run evidence.

## 9.1 Prepare the target

1. Select the intended screen or window in Capture.
2. Confirm the capture area and window size.
3. Put the application in the state expected by the node.
4. Keep the target available while testing.

Node Test requires a current screen or window context. Window actions also
perform the same foreground verification as a pipeline run.

## 9.2 Test recognition only

Use recognition-only mode first. Inspect:

- matched status and confidence;
- detected OCR text;
- every candidate and selected candidate index;
- effective ROI;
- template or color thresholds;
- annotated debug output.

When OCR detects the expected words but reports no match, check grouping,
whitespace, regex semantics, candidate index, and whether the ROI splits the
phrase across distant regions.

## 9.3 Test actions separately

Action testing is intentionally disabled without an active context. Before
each input, confirm the target is still the intended foreground window.

Use harmless test data and reversible UI states. A successful plugin dispatch
means the input was sent; it does not prove the application accepted it.

## 9.4 Read full-run evidence

The Execution Log provides concise step and Patrol outcomes. Pipeline status
retains bounded step history, errors, cleanup state, and Patrol evidence.

Enable **Save debug images** in Settings when a recognition or coordinate
problem needs visual evidence. Each attempt gets a distinct annotated file.
The setting is session-only and starts disabled when the window opens.

## 9.5 Use a focused diagnosis loop

1. Reproduce one node in Node Test.
2. Inspect the raw capture and ROI.
3. Correct recognition before changing timing.
4. Test the action with the correct target foreground.
5. Add freeze waits when loading blocks interaction.
6. Run the smallest graph path that reaches the node.
7. Run the full pipeline only after the focused case passes.

Next: [Add Patrol and recovery](10-patrol-and-recovery.md).
