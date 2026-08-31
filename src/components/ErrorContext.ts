// 全局错误 context: App 持有当前 error, 任何子组件调 useError().show(msg) 即可弹 toast
import { createContext, useContext } from "react";

export interface ErrorApi {
  error: string | null;
  show: (msg: string) => void;
  clear: () => void;
}

export const ErrorContext = createContext<ErrorApi>({
  error: null,
  show: () => {},
  clear: () => {},
});

export function useError(): ErrorApi {
  return useContext(ErrorContext);
}
