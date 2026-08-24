import { tauriBridge } from '../tauri/bridge'

const events = ['job://created', 'job://status', 'job://progress', 'job://asset-ready', 'job://failed']

export async function subscribeToJobEvents(handler: () => void) {
  if (!tauriBridge.available()) return () => {}
  const unlisten = await Promise.all(events.map(event => tauriBridge.listen(event, handler)))
  return () => unlisten.forEach(stop => stop())
}
