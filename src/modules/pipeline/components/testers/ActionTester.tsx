import type { Action } from "../../../../api/pipeline";
import ClickActionTester from "./ClickActionTester";
import SwipeActionTester from "./SwipeActionTester";
import ScrollActionTester from "./ScrollActionTester";
import LongPressActionTester from "./LongPressActionTester";
import InputTextActionTester from "./InputTextActionTester";
import ClickKeyActionTester from "./ClickKeyActionTester";
import DoNothingTester from "./DoNothingTester";
import CustomActionTester from "./CustomActionTester";

interface Props {
  actions: Action[];
}

function actionLabel(action: Action): string {
  switch (action.type) {
    case "Click": return "Click";
    case "Swipe": return "Swipe";
    case "Scroll": return "Scroll";
    case "LongPress": return "LongPress";
    case "InputText": return `InputText → "${action.input_text}"`;
    case "ClickKey": return "ClickKey";
    case "DoNothing": return "DoNothing";
    case "StopTask": return "StopTask";
    case "Custom": return `Custom: ${action.action}`;
    default: return "Unknown";
  }
}

function ActionPanel({ action }: { action: Action }) {
  switch (action.type) {
    case "Click":
      return <ClickActionTester action={action} />;
    case "Swipe":
      return <SwipeActionTester action={action} />;
    case "Scroll":
      return <ScrollActionTester action={action} />;
    case "LongPress":
      return <LongPressActionTester action={action} />;
    case "InputText":
      return <InputTextActionTester action={action} />;
    case "ClickKey":
      return <ClickKeyActionTester action={action} />;
    case "DoNothing":
      return <DoNothingTester />;
    case "StopTask":
      return (
        <div className="tester-panel">
          <div className="tester-hint">
            <span className="tester-badge noop">STOP TASK</span>
            <span>Halts the pipeline execution</span>
          </div>
        </div>
      );
    case "Custom":
      return <CustomActionTester action={action} />;
    default:
      return (
        <div className="tester-panel">
          <div className="tester-hint">Unknown action type</div>
        </div>
      );
  }
}

export default function ActionTester({ actions }: Props) {
  if (actions.length === 0) {
    return (
      <div className="tester-section-empty">
        No actions defined
      </div>
    );
  }

  if (actions.length === 1) {
    return <ActionPanel action={actions[0]} />;
  }

  return (
    <div className="tester-multi-actions">
      {actions.map((action, i) => (
        <div key={i} className="tester-action-group">
          <div className="tester-action-header">
            <span className="tester-action-index">#{i + 1}</span>
            <span className="tester-action-type">{actionLabel(action)}</span>
          </div>
          <ActionPanel action={action} />
        </div>
      ))}
    </div>
  );
}
