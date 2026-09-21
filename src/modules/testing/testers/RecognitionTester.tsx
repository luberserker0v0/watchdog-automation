import type { Recognition } from "../../../api/pipeline";
import TemplateMatchTester from "./TemplateMatchTester";
import OcrTester from "./OcrTester";
import ColorMatchTester from "./ColorMatchTester";
import DirectHitTester from "./DirectHitTester";
import CustomRecognizerTester from "./CustomRecognizerTester";

interface Props {
  recognition: Recognition;
}

export default function RecognitionTester({ recognition }: Props) {
  switch (recognition.type) {
    case "TemplateMatch":
      return <TemplateMatchTester recognition={recognition} />;
    case "OCR":
      return <OcrTester recognition={recognition} />;
    case "ColorMatch":
      return <ColorMatchTester recognition={recognition} />;
    case "DirectHit":
      return <DirectHitTester />;
    case "Custom":
      return <CustomRecognizerTester recognition={recognition} />;
    default:
      return (
        <div className="tester-panel">
          <div className="tester-hint">
            Unknown recognition type: {(recognition as Recognition).type}
          </div>
        </div>
      );
  }
}
