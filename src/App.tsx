type NavItem = {
  key: string;
  label: string;
  count?: number;
  active?: boolean;
  icon: JSX.Element;
};

type Entry = {
  id: number;
  title?: string;
  subtitle: string;
  time: string;
  chars?: string;
  dimensions?: string;
  type: 'preview' | 'text';
  image?: string;
  highlighted?: boolean;
};

const navItems: NavItem[] = [
  {
    key: 'all',
    label: '全部',
    active: true,
    icon: (
      <svg viewBox="0 0 24 24" aria-hidden="true">
        <rect x="6" y="5" width="12" height="15" rx="2" />
        <path d="M9 9h6M9 13h6M15 3v4M9 3v4" />
      </svg>
    ),
  },
  {
    key: 'text',
    label: '文本',
    icon: (
      <svg viewBox="0 0 24 24" aria-hidden="true">
        <path d="M6 7h12M12 7v10M9 17h6" />
      </svg>
    ),
  },
  {
    key: 'image',
    label: '图像',
    icon: (
      <svg viewBox="0 0 24 24" aria-hidden="true">
        <rect x="4.5" y="5.5" width="15" height="13" rx="2" />
        <circle cx="10" cy="10" r="1.25" />
        <path d="M7.5 16l3.2-3.3 2.7 2.6 2-2.1 2.3 2.8" />
      </svg>
    ),
  },
  {
    key: 'file',
    label: '文件',
    icon: (
      <svg viewBox="0 0 24 24" aria-hidden="true">
        <path d="M8 4.5h6l3 3V18a2 2 0 0 1-2 2H8a2 2 0 0 1-2-2V6.5a2 2 0 0 1 2-2Z" />
        <path d="M14 4.5v4h4" />
      </svg>
    ),
  },
  {
    key: 'starred',
    label: '收藏',
    count: 4,
    icon: (
      <svg viewBox="0 0 24 24" aria-hidden="true">
        <path d="m12 4 2.5 5.2 5.7.8-4.1 4 1 5.7L12 17l-5.1 2.7 1-5.7-4.1-4 5.7-.8Z" />
      </svg>
    ),
  },
];

const entries: Entry[] = [
  {
    id: 1,
    subtitle: '刚刚',
    time: '',
    dimensions: '3456 X 2160',
    type: 'preview',
    image: '/sample2.png',
    highlighted: true,
  },
  {
    id: 2,
    subtitle: '刚刚',
    time: '',
    dimensions: '3456 X 2160',
    type: 'preview',
    image: '/sample1.png',
  },
  {
    id: 3,
    title: '/Users/shuizhao.gh/Desktop/workspace/clipboard/src/renderer/index.html',
    subtitle: '70 字符',
    time: '1 分钟前',
    type: 'text',
  },
  {
    id: 4,
    title: 'https://github.com/ChromeDevTools/chrome-devtools-mcp/tree/main/skills',
    subtitle: '70 字符',
    time: '3 分钟前',
    type: 'text',
  },
  {
    id: 5,
    title: 'codex mcp add chrome-devtools -- npx chrome-devtools-mcp@latest',
    subtitle: '63 字符',
    time: '4 分钟前',
    type: 'text',
  },
];

function App() {
  return (
    <div className="app-shell">
      <div className="ambient ambient-left" />
      <div className="ambient ambient-right" />

      <header className="topbar">
        <div className="traffic-lights" aria-hidden="true">
          <span className="light close" />
          <span className="light minimize" />
          <span className="light maximize" />
        </div>

        <div className="brand-badge" aria-hidden="true">
          <svg viewBox="0 0 24 24">
            <path d="M6 5h12a1 1 0 0 1 1 1v12a1 1 0 0 1-1 1H6a1 1 0 0 1-1-1V6a1 1 0 0 1 1-1Z" />
            <path d="M8 10h8M8 13h6" />
            <path d="M13 8h3v3" />
          </svg>
        </div>

        <div className="search-box">
          <svg viewBox="0 0 24 24" aria-hidden="true">
            <circle cx="11" cy="11" r="5.5" />
            <path d="m15.2 15.2 4.3 4.3" />
          </svg>
          <span>搜索...</span>
        </div>

        <div className="topbar-actions">
          <button className="ghost-icon" aria-label="提醒">
            <svg viewBox="0 0 24 24">
              <path d="M12 5a5 5 0 0 0-5 5v2.6L5.8 15A1 1 0 0 0 6.7 16.5h10.6a1 1 0 0 0 .9-1.5L17 12.6V10a5 5 0 0 0-5-5Z" />
              <path d="M9.8 18a2.2 2.2 0 0 0 4.4 0" />
            </svg>
            <span className="badge-dot">1</span>
          </button>
          <button className="ghost-icon" aria-label="菜单">
            <svg viewBox="0 0 24 24">
              <path d="M5 7h14M5 12h14M5 17h14" />
            </svg>
          </button>
          <button className="ghost-icon" aria-label="置顶">
            <svg viewBox="0 0 24 24">
              <path d="m12 4 4 6h-3v5.5l2.7 3.5H8.3L11 15.5V10H8Z" />
            </svg>
          </button>
        </div>
      </header>

      <nav className="category-strip" aria-label="内容分类">
        {navItems.map((item) => (
          <button
            key={item.key}
            className={`category-item${item.active ? ' is-active' : ''}`}
            type="button"
          >
            <span className="category-icon">{item.icon}</span>
            <span className="category-label">
              {item.label}
              {item.count ? ` (${item.count})` : ''}
            </span>
          </button>
        ))}
      </nav>

      <main className="content-frame">
        <section className="entries-list">
          {entries.map((entry) => (
            <article
              key={entry.id}
              className={`entry-row entry-${entry.type}${entry.highlighted ? ' is-highlighted' : ''}`}
            >
              {entry.type === 'preview' ? (
                <>
                  <div className="entry-side-label">{entry.subtitle}</div>
                  <div className="preview-card">
                    <div className="preview-inner">
                      <img src={entry.image} alt="" />
                    </div>
                    <div className="preview-meta">
                      <button type="button" className="expand-button">
                        <svg viewBox="0 0 24 24" aria-hidden="true">
                          <path d="m6 9 6 6 6-6" />
                        </svg>
                        展开
                      </button>
                      <div className="preview-dimensions">
                        <span>{entry.dimensions}</span>
                        <span>{entry.id}</span>
                      </div>
                    </div>
                  </div>
                </>
              ) : (
                <>
                  <div className="text-content">
                    <p className="entry-title">{entry.title}</p>
                    <p className="entry-time">{entry.time}</p>
                  </div>
                  <div className="text-meta">
                    <span>{entry.subtitle}</span>
                    <span>{entry.id}</span>
                  </div>
                </>
              )}
            </article>
          ))}
        </section>

        <aside className="floating-toolbar" aria-label="右侧工具栏">
          <button type="button" className="fab fab-primary" aria-label="设备">
            <svg viewBox="0 0 24 24">
              <rect x="7" y="3.5" width="10" height="17" rx="2.5" />
              <path d="M10 6.5h4M11 17.5h2" />
            </svg>
          </button>
          <button type="button" className="fab fab-muted" aria-label="播放">
            <svg viewBox="0 0 24 24">
              <path d="m9 7 8 5-8 5Z" />
            </svg>
          </button>
          <button type="button" className="fab fab-primary" aria-label="固定">
            <svg viewBox="0 0 24 24">
              <path d="m9 5 6 0 0 4 2.8 2.7-4.3.8-.2 6.5H10.7l-.2-6.5-4.3-.8L9 9Z" />
            </svg>
          </button>
        </aside>

        <div className="utility-rail" aria-hidden="true">
          <button type="button">
            <svg viewBox="0 0 24 24">
              <path d="M7 7h10v10H7z" />
              <path d="M10 14 17 7" />
            </svg>
          </button>
          <button type="button">
            <svg viewBox="0 0 24 24">
              <circle cx="12" cy="12" r="3.2" />
              <path d="M12 4v2.2M12 17.8V20M4 12h2.2M17.8 12H20M6.5 6.5l1.6 1.6M15.9 15.9l1.6 1.6M17.5 6.5l-1.6 1.6M8.1 15.9l-1.6 1.6" />
            </svg>
          </button>
          <button type="button">
            <svg viewBox="0 0 24 24">
              <path d="M7 10h10v9H7z" />
              <path d="M9 10V7a3 3 0 0 1 6 0v3" />
            </svg>
          </button>
        </div>
      </main>
    </div>
  );
}

export default App;
