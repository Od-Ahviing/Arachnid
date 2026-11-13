import { createRequire } from 'node:module'
import { readFile } from 'node:fs/promises'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

const require = createRequire(
  process.argv[2] ? path.resolve(process.argv[2], 'package.json') : import.meta.url,
)
const sharp = require('sharp')
const root = fileURLToPath(new URL('../../', import.meta.url))
const source = await readFile(path.join(root, 'src/assets/brand/arachnid-logo.png'))
const logo = source.toString('base64')
const size = 384
const frameDuration = 40
const duration = 2400
const frames = []

const poses = [
  { time: 0, x: 1, y: 1, lift: 0, roll: 0 },
  { time: 200, x: 1, y: 1, lift: 0, roll: 0 },
  { time: 440, x: 1.08, y: 0.86, lift: 12, roll: -8 },
  { time: 740, x: 0.94, y: 1.04, lift: -34, roll: -5 },
  { time: 980, x: 1, y: 1, lift: -40, roll: 5 },
  { time: 1280, x: 1.12, y: 0.83, lift: 14, roll: 0 },
  { time: 1480, x: 0.97, y: 1.04, lift: -9, roll: -3 },
  { time: 1680, x: 1.02, y: 0.98, lift: 2, roll: 1 },
  { time: 1880, x: 1, y: 1, lift: 0, roll: 0 },
  { time: duration, x: 1, y: 1, lift: 0, roll: 0 },
]

function smooth(value) {
  return value * value * (3 - 2 * value)
}

function poseAt(time) {
  const end = poses.findIndex((pose) => pose.time > time)
  const start = poses[end - 1]
  const target = poses[end]
  const progress = smooth((time - start.time) / (target.time - start.time))
  return Object.fromEntries(
    ['x', 'y', 'lift', 'roll'].map((key) => [
      key,
      start[key] + (target[key] - start[key]) * progress,
    ]),
  )
}

for (let time = 0; time < duration; time += frameDuration) {
  const pose = poseAt(time)
  const turn = smooth(Math.max(0, Math.min(1, (time - 440) / 840))) * Math.PI * 2
  const projection = Math.cos(turn)
  const width = Math.sign(projection || 1) * Math.max(0.06, Math.abs(projection))
  const angle = (pose.roll * Math.PI) / 180
  const a = Math.cos(angle) * pose.x * width
  const b = Math.sin(angle) * pose.x * width
  const c = -Math.sin(angle) * pose.y + Math.sin(turn) * 0.12
  const d = Math.cos(angle) * pose.y
  const altitude = Math.max(0, -pose.lift) / 40
  const svg = `<svg xmlns="http://www.w3.org/2000/svg" width="${size}" height="${size}">
    <defs><filter id="soft"><feGaussianBlur stdDeviation="5"/></filter></defs>
    <rect width="100%" height="100%" fill="#0d0e10"/>
    <ellipse cx="192" cy="320" rx="${48 - altitude * 14}" ry="5"
      fill="#c63b49" opacity="${0.17 - altitude * 0.11}" filter="url(#soft)"/>
    <g transform="translate(192 ${184 + pose.lift}) matrix(${a} ${b} ${c} ${d} 0 0)">
      <image href="data:image/png;base64,${logo}" x="-150" y="-150" width="300" height="300"/>
    </g>
  </svg>`
  frames.push(await sharp(Buffer.from(svg)).removeAlpha().raw().toBuffer())
}

const output = path.join(root, 'src/assets/brand/arachnid-loading.gif')
await sharp(Buffer.concat(frames), {
  raw: { width: size, height: size * frames.length, channels: 3, pageHeight: size },
})
  .gif({ loop: 0, delay: frames.map(() => frameDuration), colours: 128, dither: 0, effort: 7 })
  .toFile(output)

const metadata = await sharp(output, { animated: true }).metadata()
if (
  metadata.pages < 2 ||
  metadata.loop !== 0 ||
  metadata.delay.reduce((a, b) => a + b, 0) !== duration
) {
  throw new Error('The encoded GIF does not preserve the intended animation')
}
console.log(`${output}: ${metadata.pages} frames, ${duration} ms`)
