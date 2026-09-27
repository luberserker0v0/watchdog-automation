import { openTargetWindow } from "../../api/target";
import { appLogger } from "../observability/services/appLogger";

export default function TargetWindowButton() {
  const handleClick = async () => {
    try {
      await openTargetWindow();
    } catch (err) {
      appLogger.error("Failed to open target window:", err);
    }
  };

  return (
    <button
      className="testbed-target-btn"
      onClick={handleClick}
      title="Open Target UI window"
    >
      Open Target
    </button>
  );
}
