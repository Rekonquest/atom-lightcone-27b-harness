#![allow(non_camel_case_types, dead_code, clippy::upper_case_acronyms)]

use std::os::raw::{c_char, c_int, c_void};

pub enum llama_model {}
pub enum llama_context {}
pub enum llama_vocab {}
pub enum ggml_tensor {}
pub enum ggml_context {}
pub enum ggml_cgraph {}

pub type llama_token = i32;
pub type llama_pos = i32;
pub type llama_seq_id = i32;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct llama_model_params {
    pub devices: *mut c_void,
    pub tensor_buft_overrides: *const c_void,
    pub n_gpu_layers: c_int,
    pub split_mode: c_int,
    pub load_mode: c_int,
    pub main_gpu: c_int,
    pub tensor_split: *const f32,
    pub progress_callback: *const c_void,
    pub progress_callback_user_data: *mut c_void,
    pub kv_overrides: *const c_void,
    pub vocab_only: bool,
    pub check_tensors: bool,
    pub use_extra_bufts: bool,
    pub no_host: bool,
    pub no_alloc: bool,
    pub load_mtp: bool,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct llama_context_params {
    pub n_ctx: u32,
    pub n_batch: u32,
    pub n_ubatch: u32,
    pub n_seq_max: u32,
    pub n_rs_seq: u32,
    pub n_outputs_max: u32,
    pub n_outputs_max_per_seq: u32,
    pub n_threads: i32,
    pub n_threads_batch: i32,
    pub ctx_type: c_int,
    pub rope_scaling_type: c_int,
    pub pooling_type: c_int,
    pub attention_type: c_int,
    pub flash_attn_type: c_int,
    pub rope_freq_base: f32,
    pub rope_freq_scale: f32,
    pub yarn_ext_factor: f32,
    pub yarn_attn_factor: f32,
    pub yarn_beta_fast: f32,
    pub yarn_beta_slow: f32,
    pub yarn_orig_ctx: u32,
    pub defrag_thold: f32,
    pub cb_eval: *const c_void,
    pub cb_eval_user_data: *mut c_void,
    pub type_k: c_int,
    pub type_v: c_int,
    pub path_kv_mean_center: *const c_char,
    pub abort_callback: *const c_void,
    pub abort_callback_data: *mut c_void,
    pub embeddings: bool,
    pub offload_kqv: bool,
    pub no_perf: bool,
    pub op_offload: bool,
    pub swa_full: bool,
    pub kv_unified: bool,
    pub samplers: *mut c_void,
    pub n_samplers: usize,
    pub ctx_other: *mut llama_context,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct llama_token_data {
    pub id: llama_token,
    pub logit: f32,
    pub p: f32,
}

#[repr(C)]
pub struct llama_token_data_array {
    pub data: *mut llama_token_data,
    pub size: usize,
    pub selected: i64,
    pub sorted: bool,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct llama_sampler_chain_params {
    pub no_perf: bool,
}

#[repr(C)]
pub struct llama_sampler {
    pub iface: *mut llama_sampler_i,
    pub ctx: *mut c_void,
}

#[repr(C)]
pub struct llama_sampler_i {
    pub name: Option<unsafe extern "C" fn(*const llama_sampler) -> *const c_char>,
    pub accept: Option<unsafe extern "C" fn(*mut llama_sampler, llama_token)>,
    pub apply: Option<unsafe extern "C" fn(*mut llama_sampler, *mut llama_token_data_array)>,
    pub reset: Option<unsafe extern "C" fn(*mut llama_sampler)>,
    pub clone: Option<unsafe extern "C" fn(*const llama_sampler) -> *mut llama_sampler>,
    pub free: Option<unsafe extern "C" fn(*mut llama_sampler)>,
    pub backend_init: Option<unsafe extern "C" fn(*mut llama_sampler, *mut c_void, u32) -> bool>,
    pub backend_accept: Option<
        unsafe extern "C" fn(
            *mut llama_sampler,
            *mut ggml_context,
            *mut ggml_cgraph,
            *mut ggml_tensor,
        ),
    >,
    pub backend_apply: Option<
        unsafe extern "C" fn(*mut llama_sampler, *mut ggml_context, *mut ggml_cgraph, *mut c_void),
    >,
    pub backend_set_input: Option<unsafe extern "C" fn(*mut llama_sampler)>,
    pub backend_reset: Option<unsafe extern "C" fn(*mut llama_sampler)>,
    pub copy_state: Option<unsafe extern "C" fn(*const llama_sampler, *mut llama_sampler)>,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct llama_batch {
    pub n_tokens: c_int,
    pub tokens: *mut llama_token,
    pub embd: *const f32,
    pub pos: *mut llama_pos,
    pub n_seq_id: *mut c_int,
    pub seq_id: *mut *mut llama_seq_id,
    pub logits: *mut i8,
}

unsafe extern "C" {
    pub fn llama_log_set(
        callback: Option<unsafe extern "C" fn(i32, *const c_char, *mut c_void)>,
        user_data: *mut c_void,
    );
    pub fn ggml_log_set(
        callback: Option<unsafe extern "C" fn(i32, *const c_char, *mut c_void)>,
        user_data: *mut c_void,
    );
    pub fn llama_backend_init();
    pub fn llama_model_default_params() -> llama_model_params;
    pub fn llama_context_default_params() -> llama_context_params;
    pub fn llama_sampler_chain_default_params() -> llama_sampler_chain_params;
    pub fn llama_model_load_from_file(
        path: *const c_char,
        params: llama_model_params,
    ) -> *mut llama_model;
    pub fn llama_model_free(model: *mut llama_model);
    pub fn llama_init_from_model(
        model: *mut llama_model,
        params: llama_context_params,
    ) -> *mut llama_context;
    pub fn llama_free(ctx: *mut llama_context);
    pub fn llama_model_get_vocab(model: *const llama_model) -> *const llama_vocab;
    pub fn llama_vocab_n_tokens(vocab: *const llama_vocab) -> i32;
    pub fn llama_vocab_is_eog(vocab: *const llama_vocab, token: llama_token) -> bool;
    pub fn llama_tokenize(
        vocab: *const llama_vocab,
        text: *const c_char,
        text_len: i32,
        tokens: *mut llama_token,
        n_tokens_max: i32,
        add_special: bool,
        parse_special: bool,
    ) -> i32;
    pub fn llama_batch_get_one(tokens: *mut llama_token, n_tokens: i32) -> llama_batch;
    pub fn llama_decode(ctx: *mut llama_context, batch: llama_batch) -> i32;
    pub fn llama_get_logits_ith(ctx: *mut llama_context, index: i32) -> *mut f32;
    pub fn llama_get_memory(ctx: *const llama_context) -> *mut c_void;
    pub fn llama_memory_seq_rm(
        memory: *mut c_void,
        seq_id: llama_seq_id,
        p0: llama_pos,
        p1: llama_pos,
    ) -> bool;
    pub fn llama_token_to_piece(
        vocab: *const llama_vocab,
        token: llama_token,
        buf: *mut c_char,
        length: i32,
        lstrip: i32,
        special: bool,
    ) -> i32;
    pub fn llama_sampler_init(iface: *mut llama_sampler_i, ctx: *mut c_void) -> *mut llama_sampler;
    pub fn llama_sampler_apply(sampler: *mut llama_sampler, cur_p: *mut llama_token_data_array);
    pub fn llama_sampler_chain_init(params: llama_sampler_chain_params) -> *mut llama_sampler;
    pub fn llama_sampler_chain_add(chain: *mut llama_sampler, sampler: *mut llama_sampler);
    pub fn llama_sampler_chain_get(chain: *mut llama_sampler, index: i32) -> *mut llama_sampler;
    pub fn llama_sampler_init_greedy() -> *mut llama_sampler;
    pub fn llama_sampler_sample(
        sampler: *mut llama_sampler,
        ctx: *mut llama_context,
        index: i32,
    ) -> llama_token;
    pub fn llama_sampler_free(sampler: *mut llama_sampler);
}
