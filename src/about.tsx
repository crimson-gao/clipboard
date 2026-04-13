import React from 'react';
import ReactDOM from 'react-dom/client';

import './about.css';
import packageJson from '../package.json';

export function AboutApp() {
  return (
    <main className="about-shell">
      <section className="about-hero">
        <div className="about-brand-mark" aria-hidden="true">
          <span className="about-brand-loop about-brand-loop-gold" />
          <span className="about-brand-loop about-brand-loop-cyan" />
          <span className="about-brand-dot about-brand-dot-gold" />
          <span className="about-brand-dot about-brand-dot-cyan" />
        </div>
        <div className="about-hero-copy">
          <p className="about-eyebrow">Clipboard</p>
          <h1>一个为 macOS 打造的轻量级剪贴板工具</h1>
          <p className="about-summary">
            使用 Tauri 2、React 和 TypeScript
            构建，专注于文本、图片与文件历史的快速查看、收藏和回贴。
          </p>
          <div className="about-version-row">
            <span className="about-chip">Version {packageJson.version}</span>
            <span className="about-chip about-chip-muted">Apple Silicon</span>
          </div>
        </div>
      </section>

      <section className="about-grid">
        <article className="about-card">
          <h2>Built With</h2>
          <ul className="about-list">
            <li>Tauri 2 desktop shell</li>
            <li>React 18 interface</li>
            <li>TypeScript + Rust backend</li>
          </ul>
        </article>

        <article className="about-card">
          <h2>Highlights</h2>
          <ul className="about-list">
            <li>菜单栏与全局快捷键呼出</li>
            <li>文本、图片、文件统一历史视图</li>
            <li>收藏、搜索、快速粘贴与 Finder 定位</li>
          </ul>
        </article>
      </section>

      <footer className="about-footer">
        <span>Clipboard Desktop</span>
        <span>Designed for a calm, native-feeling macOS workflow.</span>
      </footer>
    </main>
  );
}

ReactDOM.createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    <AboutApp />
  </React.StrictMode>,
);
