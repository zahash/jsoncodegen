export class WasiHost {
    input: Uint8Array;
    inputCursor: number;
    stdout: string[];
    stderr: string[];
    memory: WebAssembly.Memory | null;

    constructor(inputString: string) {
        this.input = new TextEncoder().encode(inputString);
        this.inputCursor = 0;
        this.stdout = [];
        this.stderr = [];
        this.memory = null; // Will be set after instantiation
    }

    // Initialize imports for the WebAssembly.instantiate
    getImports() {
        return {
            wasi_snapshot_preview1: {
                // Return exit code 0 (success)
                proc_exit: (code: number) => {
                    if (code !== 0) throw new Error(this.stderr.join(""));
                },

                // Environment variables (we provide none)
                environ_sizes_get: (
                    environCountPtr: number,
                    environBufSizePtr: number,
                ) => {
                    const view = new DataView(this.memory!.buffer);
                    view.setUint32(environCountPtr, 0, true);
                    view.setUint32(environBufSizePtr, 0, true);
                    return 0; // WASI_ESUCCESS
                },
                environ_get: (_environ: number, _environBuf: number) => {
                    return 0; // WASI_ESUCCESS
                },

                // File Descriptor Read (stdin = 0)
                fd_read: (
                    fd: number,
                    iovsPtr: number,
                    iovsLen: number,
                    nreadPtr: number,
                ) => {
                    if (fd !== 0) return 8; // WASI_EBADF (Bad file descriptor)

                    const view = new DataView(this.memory!.buffer);
                    let totalRead = 0;

                    for (let i = 0; i < iovsLen; i++) {
                        // IOVec struct is 8 bytes: [ptr (4), len (4)]
                        const iovPtr = iovsPtr + i * 8;
                        const ptr = view.getUint32(iovPtr, true);
                        const len = view.getUint32(iovPtr + 4, true);

                        // Calculate how many bytes we can actually read
                        const remainingInput =
                            this.input.length - this.inputCursor;
                        const bytesToRead = Math.min(len, remainingInput);

                        if (bytesToRead > 0) {
                            // Copy input to WASM memory
                            const src = this.input.subarray(
                                this.inputCursor,
                                this.inputCursor + bytesToRead,
                            );
                            const dst = new Uint8Array(
                                this.memory!.buffer,
                                ptr,
                                bytesToRead,
                            );
                            dst.set(src);

                            this.inputCursor += bytesToRead;
                            totalRead += bytesToRead;
                        }
                    }

                    view.setUint32(nreadPtr, totalRead, true);
                    return 0; // WASI_ESUCCESS
                },

                // File Descriptor Write (stdout = 1, stderr = 2)
                fd_write: (
                    fd: number,
                    iovsPtr: number,
                    iovsLen: number,
                    nwrittenPtr: number,
                ) => {
                    if (fd !== 1 && fd !== 2) return 8; // WASI_EBADF

                    const view = new DataView(this.memory!.buffer);
                    let totalWritten = 0;
                    const decoder = new TextDecoder();

                    for (let i = 0; i < iovsLen; i++) {
                        const iovPtr = iovsPtr + i * 8;
                        const ptr = view.getUint32(iovPtr, true);
                        const len = view.getUint32(iovPtr + 4, true);

                        // Read bytes from WASM memory
                        const bytes = new Uint8Array(
                            this.memory!.buffer,
                            ptr,
                            len,
                        );
                        const str = decoder.decode(bytes, { stream: true });

                        if (fd === 1) this.stdout.push(str);
                        if (fd === 2) this.stderr.push(str);

                        totalWritten += len;
                    }

                    view.setUint32(nwrittenPtr, totalWritten, true);
                    return 0; // WASI_ESUCCESS
                },
            },
        };
    }
}

/**
 * **Usage Example:**
 * ```javascript
 * runPlugin(
 *    "https://zahash.github.io/jsoncodegen-java-wasm32-wasip1.wasm",
 *    `{"name": "Foo", "age": 1234}`,
 * )
 *    .then((stdout) => console.log(stdout))
 *    .catch((err) => console.error(err.message));
 * ```
 */
export async function runPlugin(wasmUrl: string, inputJson: string) {
    const host = new WasiHost(inputJson);

    // instantiateStreaming requires application/wasm mime type
    const { instance } = await WebAssembly.instantiateStreaming(
        fetch(wasmUrl),
        host.getImports(),
    );

    // Link memory so the host can access it
    host.memory = instance.exports.memory as WebAssembly.Memory;

    // Run the entry point
    (instance.exports._start as Function)();
    return host.stdout.join("");
}
