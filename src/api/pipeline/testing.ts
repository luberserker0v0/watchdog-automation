import { invoke } from "@tauri-apps/api/core";
import type { Action, ActionTestContext, CandidateSelection, MatchResult, Recognition, RecognitionDevice } from "./models";

export async function testAction(action: Action, context: ActionTestContext): Promise<string> {
  return invoke<string>("test_action", { action, context });
}

export async function testRecognition(
  recognition: Recognition,
  roi: [number, number, number, number] | undefined,
  imageBytes: number[] | null,
  candidateSelection?: CandidateSelection,
): Promise<MatchResult> {
  return invoke<MatchResult>("test_recognition", { recognition, roi, imageBytes, candidateSelection });
}

export async function setRecognitionDevice(device: RecognitionDevice): Promise<string> {
  return invoke<string>("set_recognition_device", { device });
}
