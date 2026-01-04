/**
 * @file jsoncodegen-wasm32-wasip1.ts
 * A robust, zero-dependency WASI host implementation for running
 * Rust-generated WASM plugins in the browser.
 */

// ============================================================================
// PUBLIC API
// ============================================================================

/**
 * Represents a compiled WASM plugin ready to be executed.
 * * This class holds the `WebAssembly.Module` (which is stateless and cacheable)
 * and spawns fresh `WebAssembly.Instance`s for every execution.
 */
export class WasmPlugin {
  private module: WebAssembly.Module;

  /**
   * @param module - The compiled WebAssembly module.
   */
  constructor(module: WebAssembly.Module) {
    this.module = module;
  }

  /**
   * Loads and compiles a WASM plugin from a URL.
   * * Uses `instantiateStreaming` where possible for maximum performance,
   * with a fallback to `arrayBuffer` for wider compatibility.
   * * @param url - The URL of the .wasm file.
   * @returns A promise that resolves to a generic WasmPlugin.
   */
  static async load(url: string | URL): Promise<WasmPlugin> {
    const response = fetch(url);

    try {
      // Happy path: Stream compilation directly from network
      // Note: Server must serve with 'application/wasm' MIME type
      const module = await WebAssembly.compileStreaming(response);
      return new WasmPlugin(module);
    } catch (e) {
      // Fallback: Download fully, then compile
      // Useful for CDNs or local dev servers with wrong MIME types
      const buffer = await (await response).arrayBuffer();
      const module = await WebAssembly.compile(buffer);
      return new WasmPlugin(module);
    }
  }

  /**
   * Executes the plugin with the provided input string.
   * * @param input - The string to pass to the plugin's STDIN (e.g., JSON).
   * @returns The string written to STDOUT by the plugin.
   * @throws {Error} If the plugin returns a non-zero exit code or crashes.
   */
  run(input: string): string {
    const host = new WasiHost(input);

    // Synchronous instantiation is extremely fast (~microseconds)
    // because the heavy compilation work is already done in `load()`.
    const instance = new WebAssembly.Instance(this.module, host.getImports());

    // Link memory so the host can write input/read output
    host.memory = instance.exports.memory as WebAssembly.Memory;
    const start = instance.exports._start as Function;

    try {
      start();
    } catch (e) {
      // We expect the process to "crash" with a specific Exit error
      // when proc_exit is called.
      if (e instanceof WasiExit) {
        if (e.code !== 0) {
          // If exit code is error, throw the captured STDERR
          throw new Error(
            host.getStderr() || `WASM exited with code ${e.code}`,
          );
        }
        // Exit code 0 means success
      } else {
        // Actual crash (e.g. unreachable code, out of bounds)
        throw e;
      }
    }

    return host.getStdout();
  }
}

/**
 * A static manager to cache compiled plugins.
 * * Prevents re-downloading and re-compiling the same language plugin
 * multiple times.
 */
export class PluginManager {
  private static cache = new Map<string, Promise<WasmPlugin>>();

  /**
   * Gets a plugin from a specified URL, handling caching automatically.
   * * @param url - The full URL to the .wasm file.
   * @returns A promise resolving to the ready-to-use WasmPlugin.
   */
  static get(url: string): Promise<WasmPlugin> {
    if (!this.cache.has(url)) {
      // Store the Promise immediately to handle concurrent requests
      // for the same language efficiently.
      const promise = WasmPlugin.load(url);
      this.cache.set(url, promise);
    }

    return this.cache.get(url)!;
  }
}

// ============================================================================
// PRIVATE IMPLEMENTATION DETAILS
// ============================================================================

/**
 * Internal Error class used to control flow when WASM calls proc_exit.
 * Not exported to the consumer.
 */
class WasiExit extends Error {
  public code: number;
  constructor(code: number) {
    super(`WASI Exit: ${code}`);
    this.code = code;
  }
}

/**
 * The ephemeral runtime environment for a single execution.
 * * Handles File Descriptors (stdin/stdout/stderr) and memory access.
 * Not exported; consumers interact via WasmPlugin.run().
 */
class WasiHost {
  private input: Uint8Array;
  private inputCursor: number = 0;
  private stdoutChunks: string[] = [];
  private stderrChunks: string[] = [];

  // Assigned by the plugin after instantiation
  public memory: WebAssembly.Memory | null = null;

  constructor(inputString: string) {
    this.input = new TextEncoder().encode(inputString);
  }

  getStdout(): string {
    return this.stdoutChunks.join("");
  }
  getStderr(): string {
    return this.stderrChunks.join("");
  }

  /**
   * Generates the import object required by WASI P1 binaries.
   */
  getImports(): WebAssembly.Imports {
    return {
      wasi_snapshot_preview1: {
        // -----------------------------------------------------------
        // Process Lifecycle
        // -----------------------------------------------------------
        proc_exit: (code: number) => {
          throw new WasiExit(code);
        },

        // -----------------------------------------------------------
        // Environment (Mocked)
        // -----------------------------------------------------------
        environ_sizes_get: (countPtr: number, sizePtr: number) => {
          const view = this.getView();
          view.setUint32(countPtr, 0, true);
          view.setUint32(sizePtr, 0, true);
          return 0; // SUCCESS
        },
        environ_get: () => 0, // SUCCESS

        // -----------------------------------------------------------
        // File Descriptors (IO)
        // -----------------------------------------------------------
        fd_read: (
          fd: number,
          iovsPtr: number,
          iovsLen: number,
          nreadPtr: number,
        ) => {
          // 0 = STDIN
          if (fd !== 0) return 8; // EBADF (Bad File Descriptor)

          const view = this.getView();
          let totalRead = 0;

          for (let i = 0; i < iovsLen; i++) {
            // WASI iovec is 8 bytes: [pointer (4), length (4)]
            const iovPtr = iovsPtr + i * 8;
            const bufPtr = view.getUint32(iovPtr, true);
            const bufLen = view.getUint32(iovPtr + 4, true);

            const remaining = this.input.length - this.inputCursor;
            const bytesToRead = Math.min(bufLen, remaining);

            if (bytesToRead > 0) {
              // Direct memory copy from JS input -> WASM memory
              const src = this.input.subarray(
                this.inputCursor,
                this.inputCursor + bytesToRead,
              );
              new Uint8Array(this.memory!.buffer, bufPtr, bytesToRead).set(src);

              this.inputCursor += bytesToRead;
              totalRead += bytesToRead;
            }
          }

          view.setUint32(nreadPtr, totalRead, true);
          return 0; // SUCCESS
        },

        fd_write: (
          fd: number,
          iovsPtr: number,
          iovsLen: number,
          nwrittenPtr: number,
        ) => {
          // 1 = STDOUT, 2 = STDERR
          if (fd !== 1 && fd !== 2) return 8; // EBADF

          const view = this.getView();
          let totalWritten = 0;
          const decoder = new TextDecoder(); // utf-8 default

          for (let i = 0; i < iovsLen; i++) {
            const iovPtr = iovsPtr + i * 8;
            const bufPtr = view.getUint32(iovPtr, true);
            const bufLen = view.getUint32(iovPtr + 4, true);

            // Read from WASM memory -> JS string
            // We use .slice() to create a view, not a copy, for the decoder
            const bytes = new Uint8Array(this.memory!.buffer, bufPtr, bufLen);
            const chunk = decoder.decode(bytes, { stream: true });

            if (fd === 1) this.stdoutChunks.push(chunk);
            if (fd === 2) this.stderrChunks.push(chunk);

            totalWritten += bufLen;
          }

          view.setUint32(nwrittenPtr, totalWritten, true);
          return 0; // SUCCESS
        },
      },
    };
  }

  // Helper to get a DataView of current memory.
  // WASM memory can grow, detaching old buffers, so we must grab a fresh buffer every time.
  private getView(): DataView {
    if (!this.memory) throw new Error("Memory not initialized");
    return new DataView(this.memory.buffer);
  }
}
