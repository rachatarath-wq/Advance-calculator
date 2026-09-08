import { useEffect, useState } from 'react'
import { promptpayQrDataUrl } from './promptpay'

const PROMPTPAY_PHONE = '0819459785'
const BUY_ME_A_COFFEE = 'https://buymeacoffee.com/rachata.rath'

export default function CoffeePanel() {
  const [qr, setQr] = useState<string | null>(null)
  const [copied, setCopied] = useState(false)

  useEffect(() => {
    let cancelled = false
    promptpayQrDataUrl(PROMPTPAY_PHONE)
      .then((url) => {
        if (!cancelled) setQr(url)
      })
      .catch(() => {
        if (!cancelled) setQr(null)
      })
    return () => {
      cancelled = true
    }
  }, [])

  const copyPhone = async () => {
    try {
      await navigator.clipboard.writeText(PROMPTPAY_PHONE)
      setCopied(true)
      setTimeout(() => setCopied(false), 1600)
    } catch {
      /* clipboard unavailable — ignore */
    }
  }

  return (
    <main className="coffee">
      <div className="coffee-hero">
        <span className="coffee-emoji">☕</span>
        <h1>สนับสนุนค่ากาแฟ</h1>
        <p>ถ้าแอปนี้มีประโยชน์ ช่วยซื้อกาแฟให้ผมสักแก้วได้เลยครับ — ขอบคุณมาก 🙏</p>
      </div>

      <div className="coffee-cards">
        <a className="coffee-card coffee-bmc" href={BUY_ME_A_COFFEE} target="_blank" rel="noreferrer">
          <div className="coffee-card-icon">🟡</div>
          <div>
            <h2>Buy Me a Coffee</h2>
            <p>บัตรเครดิต / PayPal — ผ่าน buymeacoffee.com</p>
          </div>
          <span className="coffee-cta">เปิดลิงก์ ↗</span>
        </a>

        <div className="coffee-card coffee-promptpay">
          <div className="coffee-card-icon">📱</div>
          <div>
            <h2>PromptPay</h2>
            <p>สแกน QR หรือโอนเข้าหมายเลขโทรศัพท์</p>
          </div>
        </div>
      </div>

      <div className="coffee-qr-panel">
        <div className="coffee-qr">
          {qr ? (
            <img src={qr} alt="PromptPay QR code" width={256} height={256} />
          ) : (
            <div className="coffee-qr-fallback">…</div>
          )}
        </div>
        <div className="coffee-promptpay-info">
          <span className="coffee-label">เบอร์ PromptPay</span>
          <div className="coffee-number-row">
            <span className="coffee-number">{PROMPTPAY_PHONE}</span>
            <button className="coffee-copy" onClick={copyPhone}>
              {copied ? '✓ คัดลอกแล้ว' : 'คัดลอก'}
            </button>
          </div>
          <span className="coffee-hint">เปิดแอปธนาคาร → สแกนจ่าย → ระบุยอดตามใจชอบ</span>
        </div>
      </div>
    </main>
  )
}
