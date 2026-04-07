/**
 * Python Bridge - TypeScript wrapper for Python modules
 *
 * Provides typed interface to Python coordinator, memory, and exploration modules.
 * Uses child_process to spawn Python interpreter and communicate via JSON.
 */

import { spawn } from 'child_process'
import { pathToFileURL } from 'url'
import type { Writable } from 'stream'

export interface PythonResult<T = unknown> {
  success: boolean
  data?: T
  error?: string
}

/**
 * Execute a Python function with arguments and get JSON result
 */
export async function pythonCall<T = unknown>(
  module: string,
  function_: string,
  args: unknown[] = [],
  pythonPath = 'python3'
): Promise<PythonResult<T>> {
  const script = `
import sys
import json
import traceback

sys.path.insert(0, '${pathToFileURL('/tmp/cc-haha/src-python').href}')

try:
    from ${module} import ${function_}
    result = ${function_}(*json.loads('${JSON.stringify(args).replace(/'/g, "\\'")}'))
    print(json.dumps({"success": True, "data": result}))
except Exception as e:
    print(json.dumps({"success": False, "error": traceback.format_exc()}))
`

  return new Promise((resolve) => {
    const proc = spawn(pythonPath, ['-c', script], {
      stdio: ['pipe', 'pipe', 'pipe'],
    })

    let stdout = ''
    let stderr = ''

    proc.stdout?.on('data', (d) => (stdout += d.toString()))
    proc.stderr?.on('data', (d) => (stderr += d.toString()))

    proc.on('close', (code) => {
      if (code !== 0) {
        resolve({ success: false, error: stderr || `Exit code: ${code}` })
      } else {
        try {
          const parsed = JSON.parse(stdout)
          resolve(parsed)
        } catch {
          resolve({ success: false, error: `Invalid JSON: ${stdout}` })
        }
      }
    })
  })
}

/**
 * Execute a Python module as subprocess with IPC
 */
export class PythonSubprocess {
  private proc: ReturnType<typeof spawn> | null = null
  private messageId = 0
  private pending = new Map<number, { resolve: (v: unknown) => void; reject: (e: Error) => void }>()

  constructor(
    private module: string,
    private pythonPath = 'python3'
  ) {}

  async start(): Promise<void> {
    const script = `
import sys
import json
import traceback
from ${this.module} import *

while True:
    line = sys.stdin.readline()
    if not line:
        break
    try:
        msg = json.loads(line)
        method = msg.get('method')
        args = msg.get('args', [])
        id = msg.get('id')
        result = {'id': id}
        try:
            result['data'] = globals()[method](*args)
        except Exception as e:
            result['error'] = traceback.format_exc()
        print(json.dumps(result), flush=True)
    except Exception as e:
        print(json.dumps({'error': str(e)}), flush=True)
`

    this.proc = spawn(this.pythonPath, ['-c', script], {
      stdio: ['pipe', 'pipe', 'pipe'],
    })

    this.proc.stdout?.on('data', (data) => {
      try {
        const msg = JSON.parse(data.toString())
        const pending = this.pending.get(msg.id)
        if (pending) {
          this.pending.delete(msg.id)
          if (msg.error) {
            pending.reject(new Error(msg.error))
          } else {
            pending.resolve(msg.data)
          }
        }
      } catch {}
    })

    this.proc.on('close', () => {
      this.pending.forEach((p) => p.reject(new Error('Process exited')))
    })
  }

  async call<T = unknown>(method: string, args: unknown[] = []): Promise<T> {
    if (!this.proc) throw new Error('Not started')

    return new Promise((resolve, reject) => {
      const id = ++this.messageId
      this.pending.set(id, { resolve: resolve as (v: unknown) => void, reject })

      const msg = JSON.stringify({ id, method, args })
      this.proc?.stdin?.write(msg + '\n')
    }) as T
  }

  kill(): void {
    this.proc?.kill()
    this.proc = null
  }
}
