import './Header.css'

type Theme = 'dark' | 'light'
type Tab = 'swap' | 'pools'

interface HeaderProps {
  theme: Theme
  toggleTheme: () => void
  wallet: string | null
  connectWallet: () => void
  disconnectWallet: () => void
  activeTab: Tab
  setActiveTab: (tab: Tab) => void
}

export default function Header({
  theme, toggleTheme, wallet, connectWallet, disconnectWallet, activeTab, setActiveTab
}: HeaderProps) {
  const short = wallet ? `${wallet.slice(0, 4)}…${wallet.slice(-4)}` : null

  return (
    <header className="header">
      <div className="header-left">
        <div className="logo">
          <div className="logo-icon">
            <svg width="22" height="22" viewBox="0 0 22 22" fill="none">
              <polygon points="11,1 21,6 21,16 11,21 1,16 1,6" stroke="currentColor" strokeWidth="1.5" fill="none"/>
              <polygon points="11,5 17,8.5 17,15.5 11,19 5,15.5 5,8.5" fill="currentColor" opacity="0.15"/>
              <circle cx="11" cy="11" r="2.5" fill="currentColor"/>
            </svg>
          </div>
          <span className="logo-text">ArcDEX</span>
          <span className="logo-badge">ENCRYPTED</span>
        </div>

        <nav className="tabs">
          <button className={`tab-btn ${activeTab === 'swap' ? 'active' : ''}`} onClick={() => setActiveTab('swap')}>
            <span className="tab-icon">⇄</span>Swap
          </button>
          <button className={`tab-btn ${activeTab === 'pools' ? 'active' : ''}`} onClick={() => setActiveTab('pools')}>
            <span className="tab-icon">◈</span>Pools
          </button>
        </nav>
      </div>

      <div className="header-right">
        <div className="network-pill">
          <span className="net-dot" />
          Devnet
        </div>

        <button className="theme-toggle" onClick={toggleTheme} aria-label="Toggle theme">
          {theme === 'dark'
            ? <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2"><circle cx="12" cy="12" r="5"/><line x1="12" y1="1" x2="12" y2="3"/><line x1="12" y1="21" x2="12" y2="23"/><line x1="4.22" y1="4.22" x2="5.64" y2="5.64"/><line x1="18.36" y1="18.36" x2="19.78" y2="19.78"/><line x1="1" y1="12" x2="3" y2="12"/><line x1="21" y1="12" x2="23" y2="12"/><line x1="4.22" y1="19.78" x2="5.64" y2="18.36"/><line x1="18.36" y1="5.64" x2="19.78" y2="4.22"/></svg>
            : <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2"><path d="M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79z"/></svg>
          }
        </button>

        {wallet ? (
          <div className="wallet-connected">
            <div className="wallet-addr">
              <span className="wallet-dot" />
              <span className="wallet-short">{short}</span>
            </div>
            <button className="btn-disconnect" onClick={disconnectWallet}>Disconnect</button>
          </div>
        ) : (
          <button className="btn-connect" onClick={connectWallet}>
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5"><path d="M20 12V22H4V12"/><path d="M22 7H2v5h20V7z"/><path d="M12 22V7"/><path d="M12 7H7.5a2.5 2.5 0 0 1 0-5C11 2 12 7 12 7z"/><path d="M12 7h4.5a2.5 2.5 0 0 0 0-5C13 2 12 7 12 7z"/></svg>
            Connect Phantom
          </button>
        )}
      </div>
    </header>
  )
}