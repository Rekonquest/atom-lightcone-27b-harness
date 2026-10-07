# Local Bonsai-2 inference diagnostic, 2026-09-23

Model: LM Studio
`Ternary-Bonsai-2-27B-Uncensored-Heretic-PQ2_0.gguf`, SHA-256
`c0959544f1422c569d07ccef8911dc0d016034bc1285d91010a6630fdf4c4dda`.
The recovered model copy has the same measured digest. Existing inference
binary reported build `b1-f40a7bf` and GGUF type PQ2_0 (2.13 bpw).

Hardware observed before the probe: NVIDIA RTX 5070 Ti, 16,303 MiB total,
1,151 MiB used; 94 GiB RAM and no swap in use. No other 27B model process was
running.

The first direct `llama-cli` probe used 99 GPU layers, context 1024, 24 output
tokens, temperature zero and eight CPU threads. It loaded the model, began
generating thinking text and reported 74.2 tokens/s in its UI. On closed stdin
it remained in interactive prompt mode, flooding terminal output. The exact
process was terminated; GPU memory returned to the observed idle level. The
output is not a usable answer, and its large interactive stream is not a
benchmark artifact.

A subsequent one-shot `llama-completion` probe used the same model and GPU
settings with 16 output tokens. It exited on EOF after thinking text, without a
final answer. The binary reported 841.85 ms load, 413.66 prompt tokens/s and
73.94 generation tokens/s for this short diagnostic. These measurements are
not comparable bare-vs-Atom runs: chat-template behavior, prompt corpus,
scoring and held-out protocol were not frozen, and the new harness was not
called. Preserve this as load/generation evidence only.

The copied inference source configured and built a CUDA-enabled local
`llama-completion` with `CMAKE_CUDA_COMPILER=/usr/local/cuda/bin/nvcc` and
architecture `120a-real`. A raw `-no-cnv` 24-token prompt ended immediately;
it did not answer the question. A bounded 64-token chat-template, single-turn
run produced `2 + 2 = 4` inside `<think>` but emitted no released final answer
before its end-of-text marker. It reported 468.49 prompt tokens/s and 77.71
generation tokens/s for that smoke. These are diagnostics from the copied
binary, not a scored answer or an Atom intervention. The prompt-template and
final-answer boundary must be tested before freezing evaluation inputs.
