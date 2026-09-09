/* =========================================================================
   洪荒 · 预览服务 — 零依赖静态服务器
   以项目根（agent-workspace 上两级）为根目录 · 端口 8899
   （8000 常被其它服务占用，且若根目录不对则文档取不到）
   用途：file:// 打开界面时浏览器禁止读取本地文件，需经 http 访问
   启动：双击 预览.cmd，或 node "本文件路径"
   ========================================================================= */
const http = require("http");
const fs   = require("fs");
const path = require("path");
const { exec } = require("child_process");

const ROOT = path.resolve(__dirname, "..", "..");   // 项目根
const PORT = 8899;
const PAGE = "/artifacts/agent-workspace/index.html";

const MIME = {
  ".html": "text/html; charset=utf-8",
  ".htm":  "text/html; charset=utf-8",
  ".md":   "text/plain; charset=utf-8",
  ".js":   "text/javascript; charset=utf-8",
  ".css":  "text/css; charset=utf-8",
  ".json": "application/json; charset=utf-8",
  ".rs":   "text/plain; charset=utf-8",
  ".ps1":  "text/plain; charset=utf-8",
  ".txt":  "text/plain; charset=utf-8",
  ".toml": "text/plain; charset=utf-8",
  ".svg":  "image/svg+xml",
  ".png":  "image/png",
  ".jpg":  "image/jpeg",
  ".ico":  "image/x-icon",
  ".woff2":"font/woff2"
};

const server = http.createServer(function(req, res){
  let url;
  try { url = decodeURIComponent((req.url || "/").split("?")[0]); }
  catch(e){ res.writeHead(400); return res.end("400"); }
  if (url.endsWith("/")) url += "index.html";
  const file = path.join(ROOT, url);
  if (!file.startsWith(ROOT)){ res.writeHead(403); return res.end("403 越界"); }
  fs.readFile(file, function(err, buf){
    if (err){
      res.writeHead(404, { "Content-Type": "text/plain; charset=utf-8" });
      return res.end("404 未找到：" + url);
    }
    res.writeHead(200, { "Content-Type": MIME[path.extname(file).toLowerCase()] || "application/octet-stream" });
    res.end(buf);
  });
});

server.on("error", function(err){
  if (err && err.code === "EADDRINUSE"){
    console.log("端口 8000 已被占用（预览服务可能已在运行）");
    console.log("→ 直接打开 http://127.0.0.1:" + PORT + PAGE);
  } else {
    console.error("服务启动失败：", err && err.message);
  }
});

server.listen(PORT, function(){
  const url = "http://127.0.0.1:" + PORT + PAGE;
  console.log("洪荒 · 预览服务已启动");
  console.log("根目录：" + ROOT);
  console.log("→ " + url);
  console.log("按 Ctrl+C 停止");
  if (process.platform === "win32") exec('start "" "' + url + '"');
});
