import type { ReactNode } from "react";

export interface TabDef {
  id: string;
  label: string;
  icon?: ReactNode;
}

interface Props {
  tabs: TabDef[];
  active: string;
  onChange: (id: string) => void;
}

/**
 * 顶部 tab 条 — Apple-style 药丸 (pill) 容器内的 tab 切换.
 * 容器: 浅 parchment 背景, 内部 tab: 选中态白底 + 微阴影, 未选中态透明.
 */
export function TabBar({ tabs, active, onChange }: Props) {
  return (
    <div className="tabbar" role="tablist">
      {tabs.map((t) => {
        const isActive = t.id === active;
        return (
          <button
            key={t.id}
            role="tab"
            aria-selected={isActive}
            className={isActive ? "tab active" : "tab"}
            onClick={() => onChange(t.id)}
          >
            {t.icon}
            <span>{t.label}</span>
          </button>
        );
      })}
    </div>
  );
}
