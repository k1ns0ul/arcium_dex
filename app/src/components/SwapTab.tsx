import { useState, useEffect } from 'react'
import './SwapTab.css'

type TokenSymbol = 'SOL' | 'USDC'

interface Token {
  symbol: TokenSymbol
  name: string
  icon: string
}

const TOKENS: Record<TokenSymbol, Token> = {
  SOL:  { symbol: 'SOL',  name: 'Solana',   icon: '◎' },
  USDC: { symbol: 'USDC', name: 'USD Coin',  icon: '$' },
}

const GAS_PRESETS = [
  { label: 'Normal', value: '0',     desc: '~15s' },
  { label: 'Fast',   value: '10000', desc: '~5s'  },
  { label: 'Turbo',  value: '50000', desc: '~2s'  },
]

type SwapStatus = null | 'loading' | 'no_wallet' | 'success'
type PriceLevel = 'good' | 'neutral' | 'bad'

// Simulate a comparison delta vs Raydium (-5% to +8%) based on input amount
function getRaydiumDelta(amount: number, fromToken: TokenSymbol): number {
  // Deterministic-ish mock: varies by amount
  const base = ((amount * 7.3) % 13) - 5   // range roughly -5 to +8
  const dirBonus = fromToken === 'SOL' ? 1.5 : -1
  return parseFloat((base + dirBonus).toFixed(2))
}

function getPriceLevel(delta: number): PriceLevel {
  if (delta >= 1)   return 'good'
  if (delta >= -1)  return 'neutral'
  return 'bad'
}

interface PriceBadgeProps {
  amount: string
  fromToken: TokenSymbol
}

function PriceBadge({ amount, fromToken }: PriceBadgeProps) {
  const num = parseFloat(amount)
  if (!num || num <= 0) return null

  const delta = getRaydiumDelta(num, fromToken)
  const level = getPriceLevel(delta)

  const configs = {
    good: {
      icon: (
        <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5">
          <polyline points="20 6 9 17 4 12"/>
        </svg>
      ),
      label: `+${delta}% vs Raydium`,
      sub: 'This route is better than Raydium',
      className: 'badge-good',
    },
    neutral: {
      icon: (
        <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5">
          <line x1="5" y1="12" x2="19" y2="12"/>
        </svg>
      ),
      label: delta >= 0 ? `+${delta}% vs Raydium` : `${delta}% vs Raydium`,
      sub: 'Comparable to Raydium',
      className: 'badge-neutral',
    },
    bad: {
      icon: (
        <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5">
          <line x1="12" y1="5" x2="12" y2="19"/><line x1="5" y1="12" x2="19" y2="12"/>
        </svg>
      ),
      label: `${delta}% vs Raydium`,
      sub: 'Worse than Raydium at this size',
      className: 'badge-bad',
    },
  }

  const cfg = configs[level]

  return (
    <div className={`price-badge ${cfg.className}`}>
      <div className="badge-icon">{cfg.icon}</div>
      <div className="badge-text">
        <span className="badge-label">{cfg.label}</span>
        <span className="badge-sub">{cfg.sub}</span>
      </div>
    </div>
  )
}

interface SwapTabProps {
  wallet: string | null
}

export default function SwapTab({ wallet }: SwapTabProps) {
  const [fromToken, setFromToken] = useState<TokenSymbol>('SOL')
  const [toToken, setToToken]     = useState<TokenSymbol>('USDC')
  const [fromAmount, setFromAmount] = useState<string>('')
  const [slippage, setSlippage]     = useState<string>('0.5')
  const [customSlippage, setCustomSlippage] = useState<string>('')
  const [gasPreset, setGasPreset]   = useState<string>('0')
  const [showSettings, setShowSettings] = useState<boolean>(false)
  const [swapStatus, setSwapStatus] = useState<SwapStatus>(null)

  // Auto-clear success after 3s
  useEffect(() => {
    if (swapStatus === 'success') {
      const t = setTimeout(() => setSwapStatus(null), 3000)
      return () => clearTimeout(t)
    }
  }, [swapStatus])

  const flip = () => {
    setFromToken(toToken)
    setToToken(fromToken)
    setFromAmount('')
  }

  const handleSwap = async () => {
    if (!wallet)   { setSwapStatus('no_wallet'); return }
    if (!fromAmount || parseFloat(fromAmount) <= 0) return
    setSwapStatus('loading')
    await new Promise(r => setTimeout(r, 1800))
    setSwapStatus('success')
    setFromAmount('')
  }

  const activeSlippage = customSlippage || slippage
  const hasAmount = !!fromAmount && parseFloat(fromAmount) > 0

  return (
    <div className="swap-page">
      <div className="swap-card">
        {/* Header */}
        <div className="swap-header">
          <h2 className="swap-title">Swap</h2>
          <div className="swap-header-right">
            <div className="mev-badge">
              <svg width="11" height="11" viewBox="0 0 24 24" fill="currentColor">
                <path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"/>
              </svg>
              MEV Protected
            </div>
            <button className="settings-btn" onClick={() => setShowSettings(s => !s)}>
              <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                <circle cx="12" cy="12" r="3"/>
                <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83-2.83l.06-.06A1.65 1.65 0 0 0 4.68 15a1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 2.83-2.83l.06.06A1.65 1.65 0 0 0 9 4.68a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 2.83l-.06.06A1.65 1.65 0 0 0 19.4 9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z"/>
              </svg>
            </button>
          </div>
        </div>

        {/* Settings */}
        {showSettings && (
          <div className="settings-panel">
            <div className="settings-section">
              <label className="settings-label">Slippage Tolerance</label>
              <div className="slippage-options">
                {['0.1', '0.5', '1.0'].map(v => (
                  <button
                    key={v}
                    className={`slip-btn ${slippage === v && !customSlippage ? 'active' : ''}`}
                    onClick={() => { setSlippage(v); setCustomSlippage('') }}
                  >
                    {v}%
                  </button>
                ))}
                <div className="slip-custom">
                  <input
                    type="number"
                    placeholder="Custom"
                    value={customSlippage}
                    onChange={e => setCustomSlippage(e.target.value)}
                    min="0.01" max="50" step="0.1"
                  />
                  <span>%</span>
                </div>
              </div>
            </div>

            <div className="settings-section">
              <label className="settings-label">Transaction Priority</label>
              <div className="gas-options">
                {GAS_PRESETS.map(g => (
                  <button
                    key={g.value}
                    className={`gas-btn ${gasPreset === g.value ? 'active' : ''}`}
                    onClick={() => setGasPreset(g.value)}
                  >
                    <span className="gas-label">{g.label}</span>
                    <span className="gas-desc">{g.desc}</span>
                  </button>
                ))}
              </div>
            </div>
          </div>
        )}

        {/* From */}
        <div className="token-box">
          <div className="token-box-top">
            <span className="token-box-label">From</span>
            <span className="token-balance">Balance: —</span>
          </div>
          <div className="token-row">
            <input
              className="amount-input"
              type="number"
              placeholder="0.00"
              value={fromAmount}
              onChange={e => setFromAmount(e.target.value)}
              min="0"
            />
            <div className="token-select">
              <span className="token-icon-char">{TOKENS[fromToken].icon}</span>
              <span className="token-sym">{fromToken}</span>
            </div>
          </div>
        </div>

        <button className="flip-btn" onClick={flip}>
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5">
            <path d="M7 16V4m0 0L3 8m4-4l4 4"/>
            <path d="M17 8v12m0 0l4-4m-4 4l-4-4"/>
          </svg>
        </button>

        {/* To — amount is intentionally hidden */}
        <div className="token-box to-box-encrypted">
          <div className="token-box-top">
            <span className="token-box-label">To</span>
            <div className="encrypted-badge">
              <svg width="10" height="10" viewBox="0 0 24 24" fill="currentColor">
                <rect x="3" y="11" width="18" height="11" rx="2"/><path d="M7 11V7a5 5 0 0 1 10 0v4"/>
              </svg>
              Amount hidden
            </div>
          </div>
          <div className="token-row">
            <div className="encrypted-amount">
              <div className="enc-dots">
                <span /><span /><span /><span /><span />
              </div>
              <span className="enc-label">Encrypted by MPC</span>
            </div>
            <div className="token-select">
              <span className="token-icon-char">{TOKENS[toToken].icon}</span>
              <span className="token-sym">{toToken}</span>
            </div>
          </div>
        </div>

        {/* Price comparison badge */}
        {hasAmount && (
          <PriceBadge amount={fromAmount} fromToken={fromToken} />
        )}

        {/* Settings summary */}
        {hasAmount && (
          <div className="swap-info">
            <div className="info-row">
              <span>Slippage</span>
              <span className="info-val">{activeSlippage}%</span>
            </div>
            <div className="info-row">
              <span>Priority</span>
              <span className="info-val">{GAS_PRESETS.find(g => g.value === gasPreset)?.label ?? 'Normal'}</span>
            </div>
            <div className="info-row">
              <span>Protocol Fee</span>
              <span className="info-val">0.30%</span>
            </div>
            <div className="info-row enc-row">
              <span>Output Amount</span>
              <span className="info-val enc-info">
                <svg width="10" height="10" viewBox="0 0 24 24" fill="currentColor">
                  <rect x="3" y="11" width="18" height="11" rx="2"/><path d="M7 11V7a5 5 0 0 1 10 0v4"/>
                </svg>
                Revealed on-chain
              </span>
            </div>
          </div>
        )}

        {/* Notices */}
        {swapStatus === 'no_wallet' && (
          <div className="swap-notice warn">Connect your wallet to swap</div>
        )}
        {swapStatus === 'success' && (
          <div className="swap-notice success">
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5">
              <polyline points="20 6 9 17 4 12"/>
            </svg>
            Swap submitted — output revealed after settlement
          </div>
        )}

        <button
          className={`swap-btn ${swapStatus === 'loading' ? 'loading' : ''}`}
          onClick={handleSwap}
          disabled={swapStatus === 'loading' || !hasAmount}
        >
          {swapStatus === 'loading'
            ? <><span className="spinner" /> Processing…</>
            : wallet
              ? `Swap ${fromToken} → ${toToken}`
              : 'Connect Wallet to Swap'
          }
        </button>
      </div>

      {/* Sidebar */}
      <div className="swap-sidebar">
        <div className="mini-card route-card">
          <h4 className="mini-title">Route</h4>
          <div className="route-visual">
            <div className="route-node">{fromToken}</div>
            <div className="route-line">
              <span className="route-label">ArcDEX Pool</span>
              <svg width="60" height="2" viewBox="0 0 60 2">
                <path d="M0 1 Q30 1 60 1" stroke="var(--accent)" strokeWidth="1.5" strokeDasharray="4 2"/>
              </svg>
            </div>
            <div className="route-node">{toToken}</div>
          </div>
        </div>

        <div className="mini-card stats-card">
          <h4 className="mini-title">Market</h4>
          <div className="stat-row"><span>SOL / USDC</span><span className="stat-val">$148.92</span></div>
          <div className="stat-row"><span>24h Change</span><span className="stat-val green">+3.24%</span></div>
          <div className="stat-row"><span>Pool TVL</span><span className="stat-val">$2.4M</span></div>
          <div className="stat-row"><span>24h Volume</span><span className="stat-val">$840K</span></div>
        </div>

        <div className="mini-card mpc-card">
          <h4 className="mini-title">Privacy</h4>
          <div className="mpc-feature"><div className="mpc-dot" /><span>Amount hidden in mempool</span></div>
          <div className="mpc-feature"><div className="mpc-dot" /><span>Front-running protected</span></div>
          <div className="mpc-feature"><div className="mpc-dot" /><span>Output encrypted via Arcium</span></div>
        </div>
      </div>
    </div>
  )
}