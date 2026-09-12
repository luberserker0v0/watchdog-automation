import { useState } from "react";
import CombinedLogin from "./CombinedLogin";

export default function LoginPage() {
  const [showSuccess, setShowSuccess] = useState(false);

  return (
    <div className="target-page">
      <CombinedLogin onSuccess={() => setShowSuccess(true)} />
      {showSuccess && (
        <div className="target-notification target-notification-success" data-target="text:Login Success">
          Login successful!
        </div>
      )}
    </div>
  );
}
