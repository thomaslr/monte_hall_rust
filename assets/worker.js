self.onmessage = async (e) => {
    const { totalRuns, wasmPath, jsPath, chunkSize } = e.data;
    
    try {
        const { default: init, run_batch_wasm } = await import(jsPath);
        await init(wasmPath);
        
        let done = 0n;
        const total = BigInt(totalRuns);
        const chunk = BigInt(chunkSize);
        
        while (done < total) {
            const currentBatch = total - done < chunk ? total - done : chunk;
            const results = run_batch_wasm(currentBatch);
            done += currentBatch;
            
            self.postMessage({
                switch_wins: Number(results[0]),
                stick_wins: Number(results[1]),
                batch_done: Number(currentBatch),
                is_final: done >= total
            });
        }
    } catch (err) {
        console.error("Worker error:", err);
    }
};
