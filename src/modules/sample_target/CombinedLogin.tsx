import { useState } from "react";

function UserIcon() {
  return <svg className="target-input-icon" viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" strokeWidth="2"><path d="M20 21v-2a4 4 0 0 0-4-4H8a4 4 0 0 0-4 4v2" /><circle cx="12" cy="7" r="4" /></svg>;
}

function LockIcon() {
  return <svg className="target-input-icon" viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" strokeWidth="2"><rect x="3" y="11" width="18" height="11" rx="2" /><path d="M7 11V7a5 5 0 0 1 10 0v4" /></svg>;
}

export default function CombinedLogin({ onSuccess }: { onSuccess: () => void }) {
  const [account, setAccount] = useState("");
  const [password, setPassword] = useState("");
  const [error, setError] = useState("");
  const [showPassword, setShowPassword] = useState(false);
  const login = () => {
    if (account === "admin" && password === "123456") onSuccess();
    else setError("Invalid account or password");
  };
  return (
    <div className="target-page">
      <div className="target-card">
        <div className="target-card-logo"><svg viewBox="0 0 24 24" width="28" height="28" fill="none" stroke="currentColor" strokeWidth="2"><path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z" /></svg></div>
        <div className="target-title" data-target="text:Login">Welcome Back</div>
        <div className="target-subtitle">Sign in to continue</div>
        <div className="target-field">
          <label className="target-label" data-target="text:Account">Account</label>
          <div className="target-input-wrap"><UserIcon /><input className="target-input" data-target="input:account" type="text" placeholder="Enter account..." value={account} onChange={(event) => { setAccount(event.target.value); setError(""); }} /></div>
        </div>
        <div className="target-field">
          <label className="target-label" data-target="text:Password">Password</label>
          <div className="target-input-wrap">
            <LockIcon />
            <input className="target-input" data-target="input:password" type={showPassword ? "text" : "password"} autoComplete="new-password" placeholder="Enter password..." value={password} onChange={(event) => { setPassword(event.target.value); setError(""); }} />
            <button className="target-toggle-pw" data-target="button:toggle_password" type="button" onClick={() => setShowPassword((shown) => !shown)} aria-label={showPassword ? "Hide password" : "Show password"}>
              {showPassword ? (
                <svg viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"><path d="M17.94 17.94A10.07 10.07 0 0 1 12 20c-7 0-11-8-11-8a18.45 18.45 0 0 1 5.06-5.94M9.9 4.24A9.12 9.12 0 0 1 12 4c7 0 11 8 11 8a18.5 18.5 0 0 1-2.16 3.19m-6.72-1.07a3 3 0 1 1-4.24-4.24" /><line x1="1" y1="1" x2="23" y2="23" /></svg>
              ) : (
                <svg viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"><path d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z" /><circle cx="12" cy="12" r="3" /></svg>
              )}
            </button>
          </div>
        </div>
        {error && <div className="target-error" data-target="text:Login Fail">{error}</div>}
        <button className="target-button" data-target="button:login" onClick={login}>Login</button>
        <div className="target-hint">Demo credentials: admin / 123456</div>
      </div>
    </div>
  );
}
