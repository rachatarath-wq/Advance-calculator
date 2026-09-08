// PromptPay (EMVCo) QR payload builder + QR renderer for the "buy me a coffee"
// page. Generates a standard Thai PromptPay payload from a mobile number and
// renders it as a scannable data-URL via the `qrcode` package.

import { toDataURL } from 'qrcode'

/** CRC-16/CCITT-FALSE as required by the EMVCo tag-63 checksum. */
function crc16(data: string): string {
  let crc = 0xffff
  for (let i = 0; i < data.length; i++) {
    crc ^= data.charCodeAt(i) << 8
    for (let j = 0; j < 8; j++) {
      crc = crc & 0x8000 ? ((crc << 1) ^ 0x1021) & 0xffff : (crc << 1) & 0xffff
    }
  }
  return crc.toString(16).toUpperCase().padStart(4, '0')
}

/** EMVCo TLV field: `id` + 2-digit length + `value`. */
function tlv(id: string, value: string): string {
  return id + value.length.toString().padStart(2, '0') + value
}

/** Normalise a Thai mobile number into the 13-digit PromptPay recipient id. */
function promptpayId(phone: string): string {
  const digits = phone.replace(/\D/g, '')
  if (digits.length === 10 && digits.startsWith('0')) {
    // 08X-XXX-XXXX → "0066" + 9 digits (drop the leading 0)
    return '0066' + digits.slice(1)
  }
  if (digits.length === 9 && digits.startsWith('8')) {
    return '0066' + digits
  }
  return digits
}

/** Build the full PromptPay EMVCo QR payload string (including tag-63 CRC). */
export function promptpayPayload(phone: string): string {
  const merchant = tlv('00', 'A000000677010111') + tlv('01', promptpayId(phone))
  let payload =
    tlv('00', '01') + // payload format indicator
    tlv('01', '11') + // point-of-initiation: static
    tlv('29', merchant) + // PromptPay merchant account
    tlv('53', '764') + // transaction currency: THB
    tlv('58', 'TH') // country code
  payload += tlv('63', crc16(payload + '6304'))
  return payload
}

/** Render a PromptPay QR code for `phone` as a PNG data URL. */
export function promptpayQrDataUrl(phone: string): Promise<string> {
  return toDataURL(promptpayPayload(phone), {
    errorCorrectionLevel: 'M',
    margin: 2,
    width: 512,
    color: { dark: '#0b1220', light: '#ffffff' },
  })
}
