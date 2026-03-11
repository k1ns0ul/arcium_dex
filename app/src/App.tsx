import { useState, useEffect } from 'react'
import Header from './components/Header'
import SwapTab from './components/SwapTab'
import PoolsTab from './components/PoolsTab'
import './App.css'

type Theme = 'dark' | 'light'
type Tab = 'swap' | 'pools'

export default function App() {
  const [theme, setTheme] = useState<Theme>(
    () => (localStorage.getItem('theme') as Theme) || 'dark'
  )
  const [activeTab, setActiveTab] = useState<Tab>('swap')
  const [wallet, setWallet] = useState<string | null>(null)

  useEffect(() => {
    document.documentElement.setAttribute('data-theme', theme)
    localStorage.setItem('theme', theme)
  }, [theme])

  const toggleTheme = () => setTheme(t => (t === 'dark' ? 'light' : 'dark'))

  const connectWallet = async () => {
    const solana = (window as any).solana
    if (!solana?.isPhantom) {
      window.open('https://phantom.app/', '_blank')
      return
    }
    try {
      const resp = await solana.connect()
      setWallet(resp.publicKey.toString())
    } catch (e) {
      console.error('Wallet connect error:', e)
    }
  }

  const disconnectWallet = async () => {
    const solana = (window as any).solana
    try {
      if (solana) await solana.disconnect()
      setWallet(null)
    } catch (e) {
      console.error(e)
    }
  }

  useEffect(() => {
    const solana = (window as any).solana
    if (!solana) return
    solana.on('disconnect', () => setWallet(null))
    solana.on('connect', (pk: any) => setWallet(pk.toString()))
    return () => {
      solana.off?.('disconnect')
      solana.off?.('connect')
    }
  }, [])

  return (
    <div className="app">
      <div className="bg-grid" />
      <div className="bg-glow" />

      <Header
        theme={theme}
        toggleTheme={toggleTheme}
        wallet={wallet}
        connectWallet={connectWallet}
        disconnectWallet={disconnectWallet}
        activeTab={activeTab}
        setActiveTab={setActiveTab}
      />

      <main className="main-content">
        {activeTab === 'swap' && <SwapTab wallet={wallet} />}
        {activeTab === 'pools' && <PoolsTab wallet={wallet} />}
      </main>

      <footer className="footer">
        <span className="footer-brand">ArcDEX</span>
        <span className="footer-sep">·</span>
        <span>Powered by Arcium MPC</span>
        <span className="footer-sep">·</span>
        <span className="footer-tag">MEV-Protected</span>
      </footer>
    </div>
  )
}