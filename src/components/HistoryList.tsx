import { useEffect, useState } from "react";
import {
  deleteHistory,
  errMsg,
  formatBytes,
  formatDate,
  listHistory,
  shortUrl,
} from "../api/tauri";
import type { HistoryEntry } from "../types/tauri";
import { useError } from "./ErrorContext";

interface Props {
  /** "重下" 按钮: 把 url 推回下载 tab */
  onRedownload: (url: string) => void;
}

export function HistoryList({ onRedownload }: Props) {
  const { show } = useError();
  const [entries, setEntries] = useState<HistoryEntry[]>([]);
  const [loading, setLoading] = useState(false);

  async function refresh() {
    setLoading(true);
    try {
      const list = await listHistory(100);
      setEntries(list);
    } catch (e) {
      show("加载历史失败: " + errMsg(e));
    } finally {
      setLoading(false);
    }
  }

  useEffect(() => {
    refresh();
  }, []);

  async function handleDelete(id: number) {
    if (!confirm("确认删除这条历史?")) return;
    try {
      await deleteHistory(id);
      await refresh();
    } catch (e) {
      show("删除失败: " + errMsg(e));
    }
  }

  return (
    <div>
      <div className="card">
        <div className="row-between">
          <h3 className="card-title">下载历史</h3>
          <button className="btn" onClick={refresh} disabled={loading}>
            {loading ? "刷新中..." : "刷新"}
          </button>
        </div>
      </div>

      <div className="card card-flush">
        {entries.length === 0 ? (
          <div className="empty-state">暂无下载历史</div>
        ) : (
          <table className="table">
            <thead>
              <tr>
                <th>标题</th>
                <th>URL</th>
                <th>时间</th>
                <th>大小</th>
                <th className="col-actions"></th>
              </tr>
            </thead>
            <tbody>
              {entries.map((e) => (
                <tr key={e.id}>
                  <td title={e.title} className="col-title">
                    <div className="truncate">
                      {e.title || "(无标题)"}
                    </div>
                  </td>
                  <td className="muted" title={e.url}>
                    {shortUrl(e.url)}
                  </td>
                  <td className="muted">{formatDate(e.downloaded_at)}</td>
                  <td>{formatBytes(e.size_bytes)}</td>
                  <td>
                    <div className="row">
                      <button
                        className="btn btn-sm"
                        onClick={() => onRedownload(e.url)}
                        title="把 URL 填回下载面板"
                      >
                        重下
                      </button>
                      <button
                        className="btn btn-sm btn-danger"
                        onClick={() => handleDelete(e.id)}
                      >
                        删除
                      </button>
                    </div>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </div>
    </div>
  );
}
