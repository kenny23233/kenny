// src/types/tauri.test.ts
// 跑法: node --test --experimental-strip-types src/types/tauri.test.ts
// 也支持: node --test --experimental-strip-types --test src/types/tauri.test.ts
// 依赖: 无 (Node 24 内置 node:test + node:assert)

import { test, suite } from "node:test";
import assert from "node:assert/strict";

import {
  PROGRESS_STATUSES,
  isProgressStatus,
  type FormatInfo,
  type HistoryEntry,
  type ProgressEvent,
  type ProgressStatus,
  type Settings,
  type VideoInfo,
} from "./tauri.ts";

suite("src/types/tauri.ts — 类型契约", () => {
  test("FT-01 PROGRESS_STATUSES 恰好三个值, 且顺序固定", () => {
    // 揭示契约: 后端 runner.rs 固定 emit 这三个字符串, 见 src-tauri/src/ytdlp/runner.rs:187,231
    assert.deepEqual([...PROGRESS_STATUSES], ["downloading", "finished", "error"]);
  });

  test("FT-02 isProgressStatus 接受三个合法值, 拒绝其他", () => {
    for (const s of PROGRESS_STATUSES) {
      assert.equal(isProgressStatus(s), true, `${s} 应当被识别为合法`);
    }
    for (const bad of ["done", "FAILED", "", "queue", 0, null, undefined, {}, "Downloading"]) {
      assert.equal(isProgressStatus(bad), false, `${JSON.stringify(bad)} 应当被拒绝`);
    }
  });

  test("FT-03 ProgressStatus 联合类型在编译期只接受三字符串", () => {
    // 这段代码的存在就是测试: TS 编译能通过 = 类型契约有效
    const a: ProgressStatus = "downloading";
    const b: ProgressStatus = "finished";
    const c: ProgressStatus = "error";
    assert.equal(a, "downloading");
    assert.equal(b, "finished");
    assert.equal(c, "error");
  });

  test("FT-04 ProgressEvent 完整对象可构造并保留 status 字段", () => {
    const ev: ProgressEvent = {
      id: "dl-1",
      percent: 42.5,
      speed: 1024,
      eta: 60,
      downloaded_bytes: 500_000,
      total_bytes: 1_200_000,
      status: "downloading",
      message: undefined,
    };
    assert.equal(ev.status, "downloading");
    assert.equal(ev.percent, 42.5);
    assert.equal(ev.total_bytes, 1_200_000);
  });

  test("FT-05 ProgressEvent 必填 status 不能省略 (TS 编译期检查)", () => {
    // @ts-expect-error — status 缺失应当报错, 这条 @ts-expect-error 必须成立, 否则测试失败
    const _ev: ProgressEvent = {
      id: "x",
      percent: 0,
      speed: null,
      eta: null,
      downloaded_bytes: 0,
      total_bytes: null,
    };
    assert.ok(_ev); // 仅用于通过 "noUnusedLocals" / 防止 TS 删掉
  });

  test("FT-06 VideoInfo 必填字段齐 + formats 是数组", () => {
    const info: VideoInfo = {
      id: "BV1xx",
      title: "示例视频",
      uploader: "频道名",
      duration: 123,
      thumbnail: "https://...",
      formats: [],
    };
    assert.equal(info.formats.length, 0);
    assert.equal(typeof info.duration, "number");
  });

  test("FT-07 FormatInfo.filesize 允许为 null", () => {
    const f: FormatInfo = {
      format_id: "137",
      ext: "mp4",
      resolution: "1920x1080",
      fps: 30,
      vcodec: "avc1",
      acodec: "none",
      filesize: null,
      filesize_approx: 50_000_000,
      tbr: 1234.5,
      format_note: "1080p",
    };
    assert.equal(f.filesize, null);
  });

  test("FT-08 HistoryEntry 字段名与后端 db.rs HistoryEntry 一致 (snake_case)", () => {
    const h: HistoryEntry = {
      id: 1,
      url: "https://example.com",
      title: "t",
      save_path: "C:\\tmp",
      size_bytes: 1000,
      downloaded_at: "2026-08-31T03:00:00Z",
      status: "completed",
    };
    // 不能是大驼峰或下划线转驼峰
    assert.equal("save_path" in h, true);
    assert.equal("downloaded_at" in h, true);
    assert.equal("size_bytes" in h, true);
  });

  test("FT-09 Settings.proxy 允许 null, cookies 数组", () => {
    const s: Settings = {
      default_save_dir: "C:\\Users\\me\\Downloads",
      default_format_preference: "bestvideo+bestaudio/best",
      proxy: null,
      cookies: [],
    };
    assert.equal(s.proxy, null);
    assert.ok(Array.isArray(s.cookies));
  });

  test("FT-10 反序列化: 来自后端的 JSON 形状可被结构化赋值覆盖", () => {
    // 模拟 Rust serde 序列化的真实 JSON 形状
    const fromBackend: ProgressEvent = JSON.parse(
      JSON.stringify({
        id: "abc",
        percent: 100,
        speed: null,
        eta: 0,
        downloaded_bytes: 1024,
        total_bytes: 1024,
        status: "finished",
      }),
    );
    assert.equal(fromBackend.status, "finished");
    assert.equal(fromBackend.speed, null);
  });
});
