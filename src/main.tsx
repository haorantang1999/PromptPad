import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import "./index.css";

const container = document.getElementById("root");

if (!container) {
  // 根节点缺失说明 index.html 与本文件不匹配，属于构建配置错误而非运行时可恢复错误。
  throw new Error("找不到根节点 #root，请检查 index.html 与 src/main.tsx 是否匹配");
}

ReactDOM.createRoot(container).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
