import { openTargetWindow } from "../../api/target";

export default function TargetWindowButton() {
  const handleClick = async () => {
    try {
      await openTargetWindow();
    } catch (err) {
      console.error("Failed to open target window:", err);
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
