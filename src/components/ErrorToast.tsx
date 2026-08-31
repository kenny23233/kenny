import { useEffect } from "react";
import { useError } from "./ErrorContext";

const AUTO_DISMISS_MS = 5000;

/**
 * 顶部居中飘一个 toast, 5 秒后自动消失. 点击任意位置或 × 按钮立即关闭.
 * 白色面板 + 左侧 3px 红色 accent bar (Apple 风格), 无重投影, 轻动画入.
 */
export function ErrorToast() {
  const { error, clear } = useError();

  useEffect(() => {
    if (!error) return;
    const t = setTimeout(clear, AUTO_DISMISS_MS);
    return () => clearTimeout(t);
  }, [error, clear]);

  if (!error) return null;
  return (
    <div
      role="alert"
      className="toast"
      onClick={clear}
    >
      <span className="toast-icon">⚠</span>
      <span className="toast-body">{error}</span>
      <button
        className="toast-close"
        onClick={(e) => {
          e.stopPropagation();
          clear();
        }}
        aria-label="关闭"
      >
        ×
      </button>
    </div>
  );
}
