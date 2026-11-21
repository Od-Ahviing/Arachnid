import { locale } from './i18n'
import { ui } from './interfaceMessages'

export function formatPlaytime(seconds: number) {
  const minutes = Math.floor(Math.max(0, seconds) / 60)
  if (minutes === 0) return ui('playtimeLessMinute')
  const hours = Math.floor(minutes / 60)
  const unit = (value: number, name: 'hour' | 'minute') =>
    new Intl.NumberFormat(locale.value, {
      style: 'unit',
      unit: name,
      unitDisplay: 'short',
    }).format(value)
  return hours ? `${unit(hours, 'hour')} ${unit(minutes % 60, 'minute')}` : unit(minutes, 'minute')
}
