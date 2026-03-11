import { useState } from 'react'
import './PoolsTab.css'

interface Pool {
  id: number
  tokenA: string
  tokenB: string
  tvl: string
  apy: string
  volume: string
  fee: string
  myLiq: string
}

const MOCK_POOLS: Pool[] = [
  { id: 1, tokenA: 'SOL',  tokenB: 'USDC', tvl: '$2.4M',  apy: '18.4%', volume: '$840K', fee: '0.30%', myLiq: '$0' },
  { id: 2, tokenA: 'SOL',  tokenB: 'USDT', tvl: '$880K',  apy: '12.1%', volume: '$320K', fee: '0.30%', myLiq: '$0' },
  { id: 3, tokenA: 'USDC', tokenB: 'USDT', tvl: '$420K',  apy: '6.2%',  volume: '$190K', fee: '0.05%', myLiq: '$0' },
]

const TOKENS = ['SOL', 'USDC', 'USDT', 'BTC', 'ETH', 'BONK']

type ViewMode = 'list' | 'init' | 'manage'

interface StatusState {
  type: 'success'
  msg: string
}

type Status = null | 'loading' | 'no_wallet' | StatusState

export default function PoolsTab({ wallet }: { wallet: string | null }) {
  const [view, setView]           = useState<ViewMode>('list')
  const [selectedPool, setSelectedPool] = useState<Pool | null>(null)
  const [initForm, setInitForm]   = useState({ tokenA: 'SOL', tokenB: 'USDC', amountA: '', amountB: '' })
  const [addForm, setAddForm]     = useState({ amountA: '', amountB: '' })
  const [status, setStatus]       = useState<Status>(null)

  const fakeAction = async (msg: string) => {
    if (!wallet) { setStatus('no_wallet'); return }
    setStatus('loading')
    await new Promise(r => setTimeout(r, 1800))
    setStatus({ type: 'success', msg })
    setTimeout(() => setStatus(null), 3000)
  }

  return (
    <div className="pools-page">
      <div className="pools-topbar">
        <div className="pools-title-row">
          <h2 className="pools-title">Liquidity Pools</h2>
          <p className="pools-sub">Provide liquidity and earn fees. All reserves encrypted via MPC.</p>
        </div>
        <div className="pools-actions">
          <button className={`pools-tab-btn ${view === 'list' ? 'active' : ''}`} onClick={() => setView('list')}>
            All Pools
          </button>
          <button className={`pools-tab-btn ${view === 'init' ? 'active' : ''}`} onClick={() => setView('init')}>
            + New Pool
          </button>
        </div>
      </div>

      {view === 'list' && (
        <div className="pools-list-view">
          <div className="pools-table">
            <div className="table-head">
              <span>Pool</span>
              <span>TVL</span>
              <span>APY</span>
              <span>24h Vol</span>
              <span>Fee</span>
              <span>My Liquidity</span>
              <span></span>
            </div>
            {MOCK_POOLS.map(pool => (
              <div key={pool.id} className="table-row" onClick={() => { setSelectedPool(pool); setView('manage') }}>
                <div className="pool-pair">
                  <div className="pair-icons">
                    <div className="token-circle token-a">{pool.tokenA[0]}</div>
                    <div className="token-circle token-b">{pool.tokenB[0]}</div>
                  </div>
                  <div className="pair-info">
                    <span className="pair-name">{pool.tokenA} / {pool.tokenB}</span>
                    <span className="pair-fee">Fee: {pool.fee}</span>
                  </div>
                </div>
                <span className="cell-val">{pool.tvl}</span>
                <span className="cell-val green">{pool.apy}</span>
                <span className="cell-val">{pool.volume}</span>
                <span className="cell-val">{pool.fee}</span>
                <span className="cell-val dim">{pool.myLiq}</span>
                <button className="row-btn" onClick={e => { e.stopPropagation(); setSelectedPool(pool); setView('manage') }}>
                  Manage →
                </button>
              </div>
            ))}
          </div>

          <div className="pools-stats">
            <div className="stat-card">
              <span className="stat-label">Total TVL</span>
              <span className="stat-big">$3.7M</span>
            </div>
            <div className="stat-card">
              <span className="stat-label">24h Fees Earned</span>
              <span className="stat-big">$4,120</span>
            </div>
            <div className="stat-card">
              <span className="stat-label">Active Pools</span>
              <span className="stat-big">3</span>
            </div>
            <div className="stat-card enc-card">
              <div className="enc-icon">
                <svg width="18" height="18" viewBox="0 0 24 24" fill="currentColor">
                  <path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"/>
                </svg>
              </div>
              <span className="stat-label">Encrypted Reserves</span>
              <span className="enc-sub">via Arcium MPC</span>
            </div>
          </div>
        </div>
      )}

      {view === 'init' && (
        <div className="form-view">
          <div className="form-card">
            <div className="form-header">
              <button className="back-btn" onClick={() => setView('list')}>← Back</button>
              <h3 className="form-title">Initialize New Pool</h3>
            </div>
            <p className="form-desc">
              Create a new liquidity pool. Your reserves will be encrypted using Arcium's MPC network — amounts are never visible on-chain.
            </p>

            <div className="form-grid">
              <div className="form-group">
                <label>Token A</label>
                <select value={initForm.tokenA} onChange={e => setInitForm(f => ({ ...f, tokenA: e.target.value }))}>
                  {TOKENS.filter(t => t !== initForm.tokenB).map(t => <option key={t}>{t}</option>)}
                </select>
              </div>
              <div className="form-group">
                <label>Token B</label>
                <select value={initForm.tokenB} onChange={e => setInitForm(f => ({ ...f, tokenB: e.target.value }))}>
                  {TOKENS.filter(t => t !== initForm.tokenA).map(t => <option key={t}>{t}</option>)}
                </select>
              </div>
              <div className="form-group">
                <label>Initial Amount — {initForm.tokenA}</label>
                <div className="input-wrap">
                  <input type="number" placeholder="0.00" value={initForm.amountA}
                    onChange={e => setInitForm(f => ({ ...f, amountA: e.target.value }))} min="0" />
                  <span className="input-suffix">{initForm.tokenA}</span>
                </div>
              </div>
              <div className="form-group">
                <label>Initial Amount — {initForm.tokenB}</label>
                <div className="input-wrap">
                  <input type="number" placeholder="0.00" value={initForm.amountB}
                    onChange={e => setInitForm(f => ({ ...f, amountB: e.target.value }))} min="0" />
                  <span className="input-suffix">{initForm.tokenB}</span>
                </div>
              </div>
            </div>

            {initForm.amountA && initForm.amountB && parseFloat(initForm.amountA) > 0 && parseFloat(initForm.amountB) > 0 && (
              <div className="form-preview">
                <div className="preview-row">
                  <span>Initial Price</span>
                  <span>1 {initForm.tokenA} = {(parseFloat(initForm.amountB) / parseFloat(initForm.amountA)).toFixed(4)} {initForm.tokenB}</span>
                </div>
                <div className="preview-row"><span>Your LP Share</span><span>100% (first provider)</span></div>
                <div className="preview-row"><span>Protocol Fee</span><span>0.30% per swap</span></div>
              </div>
            )}

            {status === 'no_wallet' && <div className="form-notice warn">Connect wallet first</div>}
            {typeof status === 'object' && status?.type === 'success' && (
              <div className="form-notice success">✓ {status.msg}</div>
            )}

            <button
              className="form-btn"
              onClick={() => fakeAction('Pool initialized! Reserves encrypted via MPC.')}
              disabled={!initForm.amountA || !initForm.amountB || status === 'loading'}
            >
              {status === 'loading'
                ? <><span className="spinner" /> Initializing Pool…</>
                : `Initialize ${initForm.tokenA}/${initForm.tokenB} Pool`
              }
            </button>
          </div>
        </div>
      )}

      {view === 'manage' && selectedPool && (
        <div className="form-view">
          <div className="form-card wide">
            <div className="form-header">
              <button className="back-btn" onClick={() => setView('list')}>← All Pools</button>
              <h3 className="form-title">{selectedPool.tokenA} / {selectedPool.tokenB}</h3>
            </div>

            <div className="pool-stats-row">
              {[
                { label: 'TVL',       val: selectedPool.tvl,    cls: '' },
                { label: 'APY',       val: selectedPool.apy,    cls: 'green' },
                { label: '24h Volume',val: selectedPool.volume, cls: '' },
                { label: 'Fee Tier',  val: selectedPool.fee,    cls: '' },
              ].map(s => (
                <div className="pstat" key={s.label}>
                  <span className="pstat-label">{s.label}</span>
                  <span className={`pstat-val ${s.cls}`}>{s.val}</span>
                </div>
              ))}
              <div className="pstat">
                <span className="pstat-label">Reserves</span>
                <span className="pstat-val enc-val">
                  <svg width="11" height="11" viewBox="0 0 24 24" fill="currentColor">
                    <path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"/>
                  </svg>
                  Encrypted
                </span>
              </div>
            </div>

            <div className="manage-tabs">
              <div className="manage-section">
                <h4 className="section-title">Add Liquidity</h4>
                <div className="form-grid small">
                  <div className="form-group">
                    <label>Amount — {selectedPool.tokenA}</label>
                    <div className="input-wrap">
                      <input type="number" placeholder="0.00" value={addForm.amountA}
                        onChange={e => setAddForm(f => ({ ...f, amountA: e.target.value }))} />
                      <span className="input-suffix">{selectedPool.tokenA}</span>
                    </div>
                  </div>
                  <div className="form-group">
                    <label>Amount — {selectedPool.tokenB}</label>
                    <div className="input-wrap">
                      <input type="number" placeholder="0.00" value={addForm.amountB}
                        onChange={e => setAddForm(f => ({ ...f, amountB: e.target.value }))} />
                      <span className="input-suffix">{selectedPool.tokenB}</span>
                    </div>
                  </div>
                </div>
                {status === 'no_wallet' && <div className="form-notice warn">Connect wallet first</div>}
                {typeof status === 'object' && status?.type === 'success' && (
                  <div className="form-notice success">✓ {status.msg}</div>
                )}
                <button className="form-btn"
                  onClick={() => fakeAction('Liquidity added successfully!')}
                  disabled={!addForm.amountA || !addForm.amountB || status === 'loading'}>
                  {status === 'loading' ? <><span className="spinner" /> Adding…</> : 'Add Liquidity'}
                </button>
              </div>

              <div className="manage-section">
                <h4 className="section-title">Remove Liquidity</h4>
                <p className="section-desc">You have <strong>0 LP tokens</strong> in this pool.</p>
                <button className="form-btn secondary" disabled>No LP tokens to remove</button>
              </div>
            </div>
          </div>
        </div>
      )}
    </div>
  )
}