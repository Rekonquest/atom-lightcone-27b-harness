# Standalone inference source copy

Source: `/home/jesse/Desktop/Projects/The harness gpt-6 cant understand/inference-engine`.
This directory is a local llama.cpp source and build-configuration copy; Ordo
services and Ordo source are not part of this standalone harness.

Inference-engine identity verified on 2026-09-23: the copied source is
byte-for-byte identical (excluding Git and build artifacts) to the local
`/home/jesse/Desktop/Projects/llama.cpp-prism` checkout at commit
`f40a7bf69ba1ac1c07608fd675606fa0e090efc3`. That checkout identifies
itself as PrismML's `prism` fork. Its loader, GGUF reader, CPU kernels, and
CUDA kernels implement the fork-specific `PQ2_0` format (ggml type 142,
group size 128). The separate stock `/home/jesse/Desktop/Projects/llama.cpp`
checkout at `50f068f` lacks this format and must not be substituted for this
model. The local build reports `commit unknown` because Git metadata was not
copied; source identity comes from the checksum comparison, not that version
string. Keep the copied fork's `ggml` libraries together with its binaries.

Existing upstream build outputs and model files were not copied. The requested
uncensored Bonsai-2 PQ2_0 GGUF remains in LM Studio's model directory and was
measured at SHA-256
`c0959544f1422c569d07ccef8911dc0d016034bc1285d91010a6630fdf4c4dda`.

The existing source build `b1-f40a7bf` loaded that GGUF on the RTX 5070 Ti and
generated tokens in a bounded local diagnostic. The copied source was then
configured with CUDA 13.2 at `/usr/local/cuda/bin/nvcc` and built its own
`build-local/bin/llama-completion`. A bounded chat-template smoke of that local
binary produced the arithmetic result in its thinking channel but no released
final answer. Raw prompt mode ended immediately. Neither is an Atom-coupled
turn, a postweights hook, a bare-versus-Atom comparison, or evidence of improved
answer quality. Native decode/logit and sampler integration remain open.
