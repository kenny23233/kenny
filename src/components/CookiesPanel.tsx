import { useEffect, useState } from "react";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import {
  deleteCookies,
  errMsg,
  formatBytes,
  formatDate,
  importCookies,
  listCookies,
} from "../api/tauri";
import type { CookieInfo } from "../types/tauri";
import { useError } from "./ErrorContext";

export function CookiesPanel() {
  const { show } = useError();
  const [cookies, setCookies] = useState<CookieInfo[]>([]);
  const [loading, setLoading] = useState(false);

  async function refresh() {
    setLoading(true);
    try {
      const list = await listCookies();
      setCookies(list);
    } catch (e) {
      show("刷新失败: " + errMsg(e));
    } finally {
      setLoading(false);
    }
  }

  useEffect(() => {
    refresh();
  }, []);

  async function handleImport() {
    try {
      const file = await openDialog({
        multiple: false,
        title: "选择 cookies.txt",
        filters: [{ name: "Cookies", extensions: ["txt"] }],
      });
      if (typeof file !== "string") return;
      const domain = await importCookies(file);
      show(`已导入: ${domain}`);
      await refresh();
    } catch (e) {
      show("导入失败: " + errMsg(e));
    }
  }

  async function handleDelete(domain: string) {
    if (!confirm(`确认删除 ${domain} 的 cookies?`)) return;
    try {
      await deleteCookies(domain);
      await refresh();
    } catch (e) {
      show("删除失败: " + errMsg(e));
    }
  }

  return (
    <div>
      <div className="card">
        <div className="row-between">
          <div>
            <h3 className="card-title">已导入的 Cookies</h3>
            <div className="muted">yt-dlp 格式的 Netscape cookies.txt 文件</div>
          </div>
          <div className="row">
            <button className="btn btn-primary" onClick={handleImport}>
              导入 Cookies 文件
            </button>
            <button className="btn" onClick={refresh} disabled={loading}>
              {loading ? "刷新中..." : "刷新"}
            </button>
          </div>
        </div>
      </div>

      <div className="card card-flush">
        {cookies.length === 0 ? (
          <div className="empty-state">暂无 cookies，点上面按钮导入</div>
        ) : (
          <table className="table">
            <thead>
              <tr>
                <th>域名</th>
                <th>大小</th>
                <th>修改时间</th>
                <th className="col-actions-sm"></th>
              </tr>
            </thead>
            <tbody>
              {cookies.map((c) => (
                <tr key={c.domain}>
                  <td>
                    <div className="text-strong">{c.domain}</div>
                    <div className="muted-sm">{c.path}</div>
                  </td>
                  <td>{formatBytes(c.size_bytes)}</td>
                  <td className="muted">{formatDate(c.last_modified)}</td>
                  <td>
                    <button
                      className="btn btn-sm btn-danger"
                      onClick={() => handleDelete(c.domain)}
                    >
                      删除
                    </button>
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
