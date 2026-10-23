import { useCallback, useEffect, useRef, useState } from "react";
import type { ReactNode } from "react";
import { getConfig, setConfig } from "../api/commands";
import { AppError } from "../api/client";
import { usePadStore } from "../state/store";
import type { AppConfigView, ConfigPatch, Protocol, Provider } from "../api/types";

/**
 * 改写超时的允许区间（毫秒），与 Rust 侧 `config::OPTIMIZE_TIMEOUT_RANGE` 对齐。
 * 前端先拦一道是为了不给后端发注定被拒的请求，而不是替代后端校验。
 */
const TIMEOUT_RANGE = { min: 1_000, max: 300_000 } as const;

/**
 * 记忆注入 token 预算区间，与 Rust 侧 `config::MEMORY_TOKEN_BUDGET_MAX` 对齐。
 * 800 是 memory.md 的产品级硬上限，配置也不能把它调大。
 */
const TOKEN_BUDGET_RANGE = { min: 1, max: 800 } as const;

/**
 * 自动保存的防抖间隔（毫秒）。
 *
 * 设置表单必须自动保存：用户填到一半要去别的应用复制 API key，
 * 点外部会离开窗口，等他回来时内容必须还在。防抖是为了不把
 * 每敲一个字符都变成一次 IPC。
 */
const AUTO_SAVE_DELAY_MS = 500;

/**
 * API key 明文的长度护栏。
 *
 * 明文只在本机 config.json 与输入框之间往返，不需要很长；
 * 上限是为了别让一段超长文本变成无界的 IPC 载荷，后端另有自己的校验。
 */
const API_KEY_MAX_LENGTH = 4096;

/** provider / protocol 的兜底值：IPC 载荷不可信，拿到未知枚举值时退回默认。 */
const DEFAULT_PROVIDER: Provider = "openai_compat";
const DEFAULT_PROTOCOL: Protocol = "openai_chat_completions";

const PROVIDER_OPTIONS: ReadonlyArray<{ value: Provider; label: string }> = [
  { value: "minimax", label: "minimax（MiniMax 开放平台）" },
  { value: "openai_compat", label: "openai_compat（OpenAI 兼容服务）" },
  { value: "ollama", label: "ollama（本地模型，不需要 API key）" },
  { value: "anthropic", label: "anthropic（Anthropic 官方）" },
];

const PROTOCOL_OPTIONS: ReadonlyArray<{ value: Protocol; label: string }> = [
  { value: "openai_chat_completions", label: "OpenAI chat/completions" },
  { value: "anthropic_messages", label: "Anthropic messages" },
];

/** 接入方式预设 id。custom = 不改动任何字段。 */
type PresetId =
  | "minimax_openai"
  | "minimax_token_plan"
  | "anthropic_official"
  | "openai_compat"
  | "ollama"
  | "custom";

/**
 * 接入方式预设。
 *
 * 默认服务地址只出现在前端：Rust 侧不内置任何服务地址，
 * 配置文件里存的永远是实际生效的地址，服务迁移时改这一张表即可。
 * 预设只写 provider / protocol / endpoint，不碰模型名与 API key。
 */
const PRESET_OPTIONS: ReadonlyArray<{
  value: PresetId;
  label: string;
  provider: Provider | null;
  protocol: Protocol | null;
  endpoint: string | null;
}> = [
  {
    value: "minimax_openai",
    label: "MiniMax 开放平台（OpenAI 兼容）",
    provider: "minimax",
    protocol: "openai_chat_completions",
    endpoint: "https://api.minimaxi.com/v1",
  },
  {
    value: "minimax_token_plan",
    label: "MiniMax Token Plan（Anthropic 协议）",
    provider: "minimax",
    protocol: "anthropic_messages",
    endpoint: "https://api.minimaxi.com/anthropic",
  },
  {
    value: "anthropic_official",
    label: "Anthropic 官方（Messages 协议）",
    provider: "anthropic",
    protocol: "anthropic_messages",
    endpoint: "https://api.anthropic.com",
  },
  {
    value: "openai_compat",
    label: "OpenAI 兼容（自定义地址）",
    provider: "openai_compat",
    protocol: "openai_chat_completions",
    endpoint: "https://api.openai.com/v1",
  },
  {
    value: "ollama",
    label: "Ollama 本地（无需 API key）",
    provider: "ollama",
    protocol: "openai_chat_completions",
    endpoint: "http://localhost:11434/v1",
  },
  {
    value: "custom",
    label: "自定义（不改动任何字段）",
    provider: null,
    protocol: null,
    endpoint: null,
  },
];

/** 表单里可编辑的文本字段。 */
type TextFieldKey =
  | "endpoint"
  | "model_name"
  | "api_key_env"
  | "optimize_timeout_ms"
  | "memory_token_budget"
  | "global_shortcut";

/** 会写进配置的字段：文本框 + 两个枚举下拉。 */
type ConfigFieldKey = TextFieldKey | "provider" | "protocol";

/** 表单状态。数字用字符串持有，避免用户清空输入框时被强制改成 0。 */
interface SettingsForm {
  /** 当前接入方式预设；provider/protocol/endpoint 任一被改过就退回「自定义」。 */
  preset: PresetId;
  provider: Provider;
  protocol: Protocol;
  endpoint: string;
  model_name: string;
  api_key_env: string;
  optimize_timeout_ms: string;
  memory_token_budget: string;
  dot_click_through: boolean;
  global_shortcut: string;
  autostart_enabled: boolean;
}

const EMPTY_FORM: SettingsForm = {
  preset: "custom",
  provider: DEFAULT_PROVIDER,
  protocol: DEFAULT_PROTOCOL,
  endpoint: "",
  model_name: "",
  api_key_env: "PROMPT_PAD_API_KEY",
  optimize_timeout_ms: "30000",
  memory_token_budget: "800",
  dot_click_through: false,
  global_shortcut: "Ctrl+Q",
  autostart_enabled: false,
};

/** 字段级校验错误，键与 ConfigFieldKey 对应。 */
type FieldErrors = Partial<Record<ConfigFieldKey, string>>;

/** 只接受非负整数字符串：小数、负号、字母都判为非法。 */
function parseInteger(raw: string): number | null {
  const trimmed = raw.trim();
  if (!/^\d+$/.test(trimmed)) {
    return null;
  }
  const value = Number.parseInt(trimmed, 10);
  return Number.isSafeInteger(value) ? value : null;
}

/** 去掉地址末尾的斜杠再比，避免 "/v1" 与 "/v1/" 被认成两个预设。 */
function normalizeEndpoint(endpoint: string): string {
  return endpoint.trim().replace(/\/+$/, "");
}

/**
 * 按 provider + protocol + endpoint 三者匹配当前预设。
 * 匹配不上返回 null —— 调用方据此显示「自定义」。
 */
function matchPreset(
  provider: Provider,
  protocol: Protocol,
  endpoint: string,
): PresetId | null {
  const target = normalizeEndpoint(endpoint);
  for (const option of PRESET_OPTIONS) {
    if (option.provider === null || option.protocol === null || option.endpoint === null) {
      continue;
    }
    if (option.provider !== provider || option.protocol !== protocol) {
      continue;
    }
    if (normalizeEndpoint(option.endpoint) !== target) {
      continue;
    }
    return option.value;
  }
  return null;
}

/** 改了 provider / protocol / endpoint 之后重算预设，匹配不上就是「自定义」。 */
function withPreset(form: SettingsForm): SettingsForm {
  return {
    ...form,
    preset: matchPreset(form.provider, form.protocol, form.endpoint) ?? "custom",
  };
}

/** IPC 载荷不可信：provider 必须是已知枚举值，否则退回默认。 */
function normalizeProvider(value: unknown): Provider {
  return PROVIDER_OPTIONS.some((option) => option.value === value)
    ? (value as Provider)
    : DEFAULT_PROVIDER;
}

/** IPC 载荷不可信：protocol 必须是已知枚举值，否则退回默认。 */
function normalizeProtocol(value: unknown): Protocol {
  return PROTOCOL_OPTIONS.some((option) => option.value === value)
    ? (value as Protocol)
    : DEFAULT_PROTOCOL;
}

/** 后端配置视图 → 表单初值。 */
function toForm(view: AppConfigView): SettingsForm {
  const provider = normalizeProvider(view.provider);
  const protocol = normalizeProtocol(view.protocol);
  return {
    preset: matchPreset(provider, protocol, view.endpoint) ?? "custom",
    provider,
    protocol,
    endpoint: view.endpoint,
    model_name: view.model_name,
    api_key_env: view.api_key_env,
    optimize_timeout_ms: String(view.optimize_timeout_ms),
    memory_token_budget: String(view.memory_token_budget),
    dot_click_through: view.dot_click_through,
    global_shortcut: view.global_shortcut || "Ctrl+Q",
    autostart_enabled: view.autostart_enabled,
  };
}

/**
 * 校验规则与 Rust 侧 `set_config` / `validate_for_request` 对齐：
 * endpoint / model_name / api_key_env 非空，超时在区间内，token 预算不破 800 上限。
 */
function validate(form: SettingsForm): FieldErrors {
  const errors: FieldErrors = {};

  if (!PROVIDER_OPTIONS.some((option) => option.value === form.provider)) {
    errors.provider = "提供方取值不合法";
  }
  if (!PROTOCOL_OPTIONS.some((option) => option.value === form.protocol)) {
    errors.protocol = "协议取值不合法";
  }

  const endpoint = form.endpoint.trim();
  if (endpoint.length === 0) {
    errors.endpoint = "模型服务地址不能为空";
  } else if (!endpoint.startsWith("http://") && !endpoint.startsWith("https://")) {
    errors.endpoint = "模型服务地址必须以 http:// 或 https:// 开头";
  }

  if (form.model_name.trim().length === 0) {
    errors.model_name = "模型名不能为空";
  }

  const apiKeyEnv = form.api_key_env.trim();
  if (apiKeyEnv.length > 0 && !/^[A-Za-z_][A-Za-z0-9_]*$/.test(apiKeyEnv)) {
    // 与 Rust 侧 `config::is_env_var_name` 同一条规则。
    // 提前拦下来是给用户看就地提示，而不是等后端回一条 invalid_input。
    errors.api_key_env = "环境变量名只能包含字母、数字和下划线，且不能以数字开头";
  }

  const timeout = parseInteger(form.optimize_timeout_ms);
  if (timeout === null) {
    errors.optimize_timeout_ms = `改写超时必须是整数（${TIMEOUT_RANGE.min}–${TIMEOUT_RANGE.max} 毫秒）`;
  } else if (timeout < TIMEOUT_RANGE.min) {
    errors.optimize_timeout_ms = `改写超时不能小于 ${TIMEOUT_RANGE.min} 毫秒`;
  } else if (timeout > TIMEOUT_RANGE.max) {
    errors.optimize_timeout_ms = `改写超时不能大于 ${TIMEOUT_RANGE.max} 毫秒`;
  }

  const budget = parseInteger(form.memory_token_budget);
  if (budget === null) {
    errors.memory_token_budget = `记忆 token 预算必须是整数（${TOKEN_BUDGET_RANGE.min}–${TOKEN_BUDGET_RANGE.max}）`;
  } else if (budget < TOKEN_BUDGET_RANGE.min) {
    errors.memory_token_budget = `记忆 token 预算不能小于 ${TOKEN_BUDGET_RANGE.min}`;
  } else if (budget > TOKEN_BUDGET_RANGE.max) {
    errors.memory_token_budget = `记忆 token 预算不能超过 ${TOKEN_BUDGET_RANGE.max}：这是 memory.md 的硬上限`;
  }

  const shortcut = form.global_shortcut.trim();
  if (shortcut.length === 0 || !shortcut.includes("+")) {
    errors.global_shortcut = "热键应形如 Ctrl+Q";
  }

  return errors;
}

/**
 * 只把用户真正改过、且**当前取值合法**的字段放进补丁，其余传 null
 * （后端是补丁语义，null 表示不修改）。
 * embedding_model 本期只读，永远传 null：换模型必须重建全库索引。
 *
 * 逐字段判断合法性是自动保存的前提：用户正在输入 endpoint 时，
 * 中间态（如 "h"）不合法，不能写进配置 —— 否则配置会被半截值污染。
 *
 * apiKey 为 null 表示不碰 API key；空字符串表示清除已存的本地明文。
 */
function buildPatch(
  form: SettingsForm,
  baseline: AppConfigView | null,
  errors: FieldErrors,
  apiKey: string | null,
): ConfigPatch {
  const endpoint = form.endpoint.trim();
  const modelName = form.model_name.trim();
  const apiKeyEnv = form.api_key_env.trim();
  const timeout = parseInteger(form.optimize_timeout_ms) ?? 0;
  const budget = parseInteger(form.memory_token_budget) ?? 0;

  const differs = (
    current: string | number | boolean,
    original: string | number | boolean | undefined,
  ) => current !== original;

  return {
    provider: differs(form.provider, baseline?.provider) ? form.provider : null,
    protocol: differs(form.protocol, baseline?.protocol) ? form.protocol : null,
    endpoint: differs(endpoint, baseline?.endpoint) && !errors.endpoint ? endpoint : null,
    model_name: differs(modelName, baseline?.model_name) && !errors.model_name ? modelName : null,
    embedding_model: null,
    optimize_timeout_ms:
      differs(timeout, baseline?.optimize_timeout_ms) && !errors.optimize_timeout_ms
        ? timeout
        : null,
    api_key_env: differs(apiKeyEnv, baseline?.api_key_env) && !errors.api_key_env ? apiKeyEnv : null,
    api_key: apiKey,
    memory_token_budget:
      differs(budget, baseline?.memory_token_budget) && !errors.memory_token_budget ? budget : null,
    dot_click_through: differs(form.dot_click_through, baseline?.dot_click_through)
      ? form.dot_click_through
      : null,
    global_shortcut:
      differs(form.global_shortcut.trim(), baseline?.global_shortcut) && !errors.global_shortcut
        ? form.global_shortcut.trim()
        : null,
    autostart_enabled: differs(form.autostart_enabled, baseline?.autostart_enabled)
      ? form.autostart_enabled
      : null,
  };
}

/** 补丁里是否还有内容。 */
function isEmptyPatch(patch: ConfigPatch): boolean {
  return Object.values(patch).every((value) => value === null);
}

/** 把异常收敛成可展示的文案：错误码来自 client.ts 的统一约定。 */
function readErrorMessage(err: unknown): string {
  if (err instanceof AppError) {
    return `[${err.code}] ${err.message}`;
  }
  if (err instanceof Error) {
    return err.message;
  }
  return "未知错误";
}

const INPUT_CLASS =
  "rounded-xl border border-[var(--pad-border)] bg-[var(--pad-surface)] px-2.5 py-1.5 text-[12px] leading-relaxed text-[var(--pad-text)] placeholder:text-[var(--pad-text-tertiary)] focus:border-[var(--pad-accent)] focus:outline-none focus:shadow-[0_0_0_3px_var(--pad-accent-soft)]";

interface FieldProps {
  label: string;
  hint?: string | undefined;
  error?: string | undefined;
  children: ReactNode;
}

/** 表单字段外壳：标签 + 控件 + 提示 + 就地错误。 */
function Field({ label, hint, error, children }: FieldProps) {
  return (
    <label className="flex min-w-0 flex-col gap-1">
      <span className="text-[11px] font-medium text-[var(--pad-text-secondary)]">{label}</span>
      {children}
      {hint ? (
        <span className="text-[10px] leading-tight text-[var(--pad-text-tertiary)]">{hint}</span>
      ) : null}
      {error ? (
        <span className="text-[10px] leading-tight text-[var(--pad-danger)]">{error}</span>
      ) : null}
    </label>
  );
}

/**
 * 设置视图。
 *
 * 改动**自动保存**：改完字段停手 500ms 就写盘，离开字段时立即写盘。
 * 手动「保存」按钮保留，作为「现在就写」的显式入口。
 *
 * API key 明文可以直填，但只落在本机 config.json 里：后端永不回显，
 * 所以明文只存在本地 state，挂载时从空值开始，也不被保存后的回显覆盖。
 */
export function Settings() {
  const setView = usePadStore((s) => s.setView);
  const setStoreConfig = usePadStore((s) => s.setConfig);

  const [form, setForm] = useState<SettingsForm>(EMPTY_FORM);
  const [errors, setErrors] = useState<FieldErrors>({});
  const [loadError, setLoadError] = useState<string | null>(null);
  const [saveError, setSaveError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);
  const [saved, setSaved] = useState(false);
  const [apiKeyPresent, setApiKeyPresent] = useState(false);
  const [embeddingModel, setEmbeddingModel] = useState("");

  // API key 明文：后端不回显，状态完全由前端持有。
  const [apiKeyInput, setApiKeyInput] = useState("");
  const [apiKeyVisible, setApiKeyVisible] = useState(false);
  const [apiKeyError, setApiKeyError] = useState<string | null>(null);

  // 基线快照：用来判断「用户到底改了哪些字段」。
  const baselineRef = useRef<AppConfigView | null>(null);
  // 最新表单：防抖定时器回调里读它，保证存的是用户最后输入的值。
  const formRef = useRef<SettingsForm>(EMPTY_FORM);
  // 最新 API key 明文与「是否被用户改过」标记。
  const apiKeyRef = useRef<{ value: string; dirty: boolean }>({ value: "", dirty: false });
  const timerRef = useRef<number | null>(null);

  useEffect(() => {
    let disposed = false;

    void getConfig()
      .then((view) => {
        if (disposed) return;
        baselineRef.current = view;
        const next = toForm(view);
        formRef.current = next;
        setForm(next);
        setApiKeyPresent(view.api_key_present);
        setEmbeddingModel(view.embedding_model);
      })
      .catch((err: unknown) => {
        if (disposed) return;
        setLoadError(readErrorMessage(err));
      });

    return () => {
      disposed = true;
    };
  }, []);

  // 卸载时把待保存的改动落盘：面板收起时表单会整个卸载，
  // 那一次还没到防抖时间的改动不能丢。
  useEffect(() => {
    return () => {
      if (timerRef.current !== null) {
        window.clearTimeout(timerRef.current);
        timerRef.current = null;
        persistRef.current("auto");
      }
    };
  }, []);

  /** 把当前表单写盘。manual = 用户点了「保存」，auto = 自动保存。 */
  const persist = useCallback((origin: "auto" | "manual") => {
    const current = formRef.current;
    if (baselineRef.current === null && origin === "auto") {
      return;
    }
    const fieldErrors = validate(current);
    setErrors(fieldErrors);

    const apiKeyState = apiKeyRef.current;
    const apiKeyValue = apiKeyState.value.trim();
    // 明文只在用户真的改过、且不是把已配置的 key 清空时才写。
    // 空框是「永不回显」的正常样子，失焦发空串会把磁盘密钥清掉。
    const keyError =
      apiKeyState.dirty && apiKeyValue.length > API_KEY_MAX_LENGTH
        ? `API key 过长（上限 ${API_KEY_MAX_LENGTH} 字符）`
        : null;
    setApiKeyError(keyError);
    const shouldWriteKey = apiKeyState.dirty && keyError === null && apiKeyValue.length > 0;

    const patch = buildPatch(
      current,
      baselineRef.current,
      fieldErrors,
      shouldWriteKey ? apiKeyValue : null,
    );
    if (isEmptyPatch(patch)) {
      if (origin === "manual") {
        setSaveError("没有需要保存的改动");
      }
      return;
    }

    setSaving(true);
    setSaveError(null);

    void setConfig(patch)
      .then((view) => {
        // 用后端回显的规范化结果刷新基线与表单，避免界面停留在未保存的中间态。
        // 预设沿用用户当前选择：回显里的地址可能被后端规范化过，
        // 但用户挑的是哪个接入方式，不该被回显改写。
        baselineRef.current = view;
        setStoreConfig(view);
        const next = { ...toForm(view), preset: current.preset };
        formRef.current = next;
        setForm(next);
        setApiKeyPresent(view.api_key_present);
        setEmbeddingModel(view.embedding_model);
        // 读配置失败的提示在保存成功后已经过期：配置已写入，撤掉它免得误导。
        setLoadError(null);
        setSaving(false);
        setSaved(true);
        if (patch.api_key !== null) {
          // 明文已落盘，撤掉 dirty：后续保存别把同一个 key 反复写一遍。
          apiKeyRef.current = { ...apiKeyRef.current, dirty: false };
        }
      })
      .catch((err: unknown) => {
        setSaving(false);
        setSaved(false);
        setSaveError(readErrorMessage(err));
      });
  }, [setStoreConfig]);

  const persistRef = useRef(persist);
  persistRef.current = persist;

  /** 改动字段：立刻更新表单，标记未保存，并安排一次自动保存。 */
  const applyForm = useCallback((next: SettingsForm) => {
    formRef.current = next;
    setForm(next);
    setSaved(false);
    setSaveError(null);
  }, []);

  /** 防抖自动保存。 */
  const scheduleAutoSave = useCallback(() => {
    if (timerRef.current !== null) {
      window.clearTimeout(timerRef.current);
    }
    timerRef.current = window.setTimeout(() => {
      timerRef.current = null;
      persistRef.current("auto");
    }, AUTO_SAVE_DELAY_MS);
  }, []);

  /** 离开字段：不等防抖，立刻写盘。 */
  const flushAutoSave = useCallback(() => {
    if (timerRef.current !== null) {
      window.clearTimeout(timerRef.current);
      timerRef.current = null;
    }
    persistRef.current("auto");
  }, []);

  const updateField = useCallback(
    (key: TextFieldKey, value: string, autoSave: "debounce" | "none" = "debounce") => {
      const next = { ...formRef.current, [key]: value };
      // endpoint 参与预设匹配，改完要重算；其余字段与预设无关。
      applyForm(key === "endpoint" ? withPreset(next) : next);
      if (autoSave === "debounce") {
        scheduleAutoSave();
      }
    },
    [applyForm, scheduleAutoSave],
  );

  /** 选接入方式预设：只改 provider / protocol / endpoint，不碰模型名与 API key。 */
  const handlePresetChange = useCallback(
    (value: PresetId) => {
      const option = PRESET_OPTIONS.find((item) => item.value === value);
      if (!option) return;
      const current = formRef.current;
      if (option.provider === null || option.protocol === null || option.endpoint === null) {
        // 自定义：不改动任何字段，只把下拉停在「自定义」。
        applyForm({ ...current, preset: "custom" });
        return;
      }
      applyForm({
        ...current,
        preset: option.value,
        provider: option.provider,
        protocol: option.protocol,
        endpoint: option.endpoint,
      });
      flushAutoSave();
    },
    [applyForm, flushAutoSave],
  );

  const handleProviderChange = useCallback(
    (value: Provider) => {
      applyForm(withPreset({ ...formRef.current, provider: value }));
      flushAutoSave();
    },
    [applyForm, flushAutoSave],
  );

  /**
   * 改协议时，若当前提供方 + 新协议能对上某条预设，同步那条预设的 endpoint。
   * MiniMax 同时有 OpenAI 兼容端点和 Anthropic 兼容端点，地址不同；
   * 找不到匹配预设则只改协议、不动地址。
   */
  const handleProtocolChange = useCallback(
    (value: Protocol) => {
      const current = formRef.current;
      const match = PRESET_OPTIONS.find(
        (item) => item.provider === current.provider && item.protocol === value && item.endpoint,
      );
      applyForm(
        withPreset({
          ...current,
          protocol: value,
          endpoint: match?.endpoint ?? current.endpoint,
        }),
      );
      flushAutoSave();
    },
    [applyForm, flushAutoSave],
  );

  /** 改 API key 明文：只存在本地 state，后端不回显。 */
  const handleApiKeyChange = useCallback(
    (value: string) => {
      apiKeyRef.current = { value, dirty: true };
      setApiKeyInput(value);
      setApiKeyError(
        value.trim().length > API_KEY_MAX_LENGTH
          ? `API key 过长（上限 ${API_KEY_MAX_LENGTH} 字符）`
          : null,
      );
      setSaved(false);
      setSaveError(null);
      scheduleAutoSave();
    },
    [scheduleAutoSave],
  );

  const handleSave = useCallback(() => {
    flushAutoSave();
  }, [flushAutoSave]);

  return (
    <div className="flex min-h-0 flex-1 flex-col">
      <div className="min-h-0 flex-1 overflow-y-auto p-3">
        {loadError ? (
          <p className="mb-2 rounded-xl border border-[rgb(255_149_0/0.25)] bg-[rgb(255_149_0/0.08)] px-2.5 py-1.5 text-[10px] leading-tight text-[var(--pad-warning)]">
            {loadError}
          </p>
        ) : null}
        {saveError ? (
          <p className="mb-2 rounded-xl border border-[rgb(255_59_48/0.25)] bg-[rgb(255_59_48/0.08)] px-2.5 py-1.5 text-[10px] leading-tight text-[var(--pad-danger)]">
            {saveError}
          </p>
        ) : null}

        <div className="flex flex-col gap-2.5">
          <Field
            label="接入方式"
            hint="选预设会改写提供方、协议与服务地址；模型名与 API key 不动"
          >
            <select
              value={form.preset}
              onChange={(event) => {
                const next = PRESET_OPTIONS.find((option) => option.value === event.target.value);
                if (next) {
                  handlePresetChange(next.value);
                }
              }}
              className={INPUT_CLASS}
            >
              {PRESET_OPTIONS.map((option) => (
                <option key={option.value} value={option.value}>
                  {option.label}
                </option>
              ))}
            </select>
          </Field>

          <Field label="模型提供方" error={errors.provider}>
            <select
              value={form.provider}
              onChange={(event) => {
                const next = PROVIDER_OPTIONS.find((option) => option.value === event.target.value);
                if (next) {
                  handleProviderChange(next.value);
                }
              }}
              className={INPUT_CLASS}
            >
              {PROVIDER_OPTIONS.map((option) => (
                <option key={option.value} value={option.value}>
                  {option.label}
                </option>
              ))}
            </select>
          </Field>

          <Field
            label="协议 protocol"
            hint="与提供方相互独立。若当前提供方有对应预设，会同步那条预设的服务地址"
            error={errors.protocol}
          >
            <select
              value={form.protocol}
              onChange={(event) => {
                const next = PROTOCOL_OPTIONS.find((option) => option.value === event.target.value);
                if (next) {
                  handleProtocolChange(next.value);
                }
              }}
              className={INPUT_CLASS}
            >
              {PROTOCOL_OPTIONS.map((option) => (
                <option key={option.value} value={option.value}>
                  {option.label}
                </option>
              ))}
            </select>
          </Field>

          <Field
            label="模型服务地址 endpoint"
            hint="填到版本段即可（程序自动补 /chat/completions 或 /messages）；也可直接粘贴完整请求地址，原样使用"
            error={errors.endpoint}
          >
            <input
              type="text"
              value={form.endpoint}
              onChange={(event) => updateField("endpoint", event.target.value)}
              onBlur={flushAutoSave}
              placeholder="https://api.example.com/v1"
              spellCheck={false}
              className={INPUT_CLASS}
            />
          </Field>

          <Field label="模型名 model_name" error={errors.model_name}>
            <input
              type="text"
              value={form.model_name}
              onChange={(event) => updateField("model_name", event.target.value)}
              onBlur={flushAutoSave}
              placeholder="例如 gpt-4o-mini"
              spellCheck={false}
              className={INPUT_CLASS}
            />
          </Field>

          <Field
            label="API key"
            hint="直接粘贴即可。明文只保存在本机 %APPDATA% 下的 config.json，不写入 git 仓库与 memory/"
            error={apiKeyError ?? undefined}
          >
            <div className="flex items-center gap-2">
              <input
                type={apiKeyVisible ? "text" : "password"}
                value={apiKeyInput}
                onChange={(event) => handleApiKeyChange(event.target.value)}
                onBlur={flushAutoSave}
                placeholder="粘贴 API key"
                spellCheck={false}
                autoComplete="off"
                className={`${INPUT_CLASS} min-w-0 flex-1`}
              />
              <button
                type="button"
                onClick={() => setApiKeyVisible((visible) => !visible)}
                className="shrink-0 rounded-full border border-[var(--pad-border)] px-2.5 py-1 text-[10px] font-medium text-[var(--pad-text-secondary)] transition-colors hover:bg-[var(--pad-surface)] focus:outline-none focus-visible:ring-2 focus-visible:ring-[var(--pad-accent-ring)]"
              >
                {apiKeyVisible ? "隐藏" : "显示"}
              </button>
              <span
                className={
                  form.provider === "ollama" || apiKeyPresent
                    ? "shrink-0 text-[10px] font-medium text-[var(--pad-success)]"
                    : "shrink-0 text-[10px] font-medium text-[var(--pad-warning)]"
                }
              >
                {form.provider === "ollama" ? "本地无需 key" : apiKeyPresent ? "已配置" : "未配置"}
              </span>
            </div>
          </Field>

          <Field
            label="API key 环境变量名"
            hint="备选来源：本地未填明文时，若该环境变量存在则使用环境变量里的值"
            error={errors.api_key_env}
          >
            <input
              type="text"
              value={form.api_key_env}
              onChange={(event) => updateField("api_key_env", event.target.value)}
              onBlur={flushAutoSave}
              placeholder="PROMPT_PAD_API_KEY"
              spellCheck={false}
              className={INPUT_CLASS}
            />
          </Field>

          <div className="grid grid-cols-2 gap-2">
            <Field
              label="改写超时（毫秒）"
              hint={`${TIMEOUT_RANGE.min}–${TIMEOUT_RANGE.max}`}
              error={errors.optimize_timeout_ms}
            >
              <input
                type="number"
                inputMode="numeric"
                min={TIMEOUT_RANGE.min}
                max={TIMEOUT_RANGE.max}
                step={1000}
                value={form.optimize_timeout_ms}
                onChange={(event) =>
                  updateField("optimize_timeout_ms", event.target.value, "none")
                }
                onBlur={flushAutoSave}
                className={INPUT_CLASS}
              />
            </Field>

            <Field
              label="记忆 token 预算"
              hint={`${TOKEN_BUDGET_RANGE.min}–${TOKEN_BUDGET_RANGE.max}（硬上限）`}
              error={errors.memory_token_budget}
            >
              <input
                type="number"
                inputMode="numeric"
                min={TOKEN_BUDGET_RANGE.min}
                max={TOKEN_BUDGET_RANGE.max}
                step={50}
                value={form.memory_token_budget}
                onChange={(event) =>
                  updateField("memory_token_budget", event.target.value, "none")
                }
                onBlur={flushAutoSave}
                className={INPUT_CLASS}
              />
            </Field>
          </div>

          <Field
            label="全局热键"
            hint="改键立即重绑。形如 Ctrl+Q、Ctrl+Shift+Space"
            error={errors.global_shortcut}
          >
            <input
              type="text"
              value={form.global_shortcut}
              onChange={(event) => updateField("global_shortcut", event.target.value)}
              onBlur={flushAutoSave}
              placeholder="Ctrl+Q"
              spellCheck={false}
              className={INPUT_CLASS}
            />
          </Field>

          <Field label="嵌入模型 embedding_model" hint="换模型必须重建全库索引，本期只读">
            <div className="flex items-center justify-between rounded-xl border border-[var(--pad-border)] bg-[var(--pad-surface)] px-2.5 py-1.5">
              <span className="truncate text-[12px] text-[var(--pad-text)]">
                {embeddingModel.length > 0 ? embeddingModel : "—"}
              </span>
              <span className="shrink-0 text-[10px] text-[var(--pad-text-tertiary)]">只读</span>
            </div>
          </Field>

          <div className="flex items-start gap-2 rounded-xl border border-[var(--pad-border)] bg-[var(--pad-surface)] px-2.5 py-2">
            <input
              id="autostart_enabled"
              type="checkbox"
              checked={form.autostart_enabled}
              onChange={(event) => {
                applyForm({ ...formRef.current, autostart_enabled: event.target.checked });
                flushAutoSave();
              }}
              className="mt-0.5 h-3.5 w-3.5 shrink-0 accent-[var(--pad-accent)]"
            />
            <label
              htmlFor="autostart_enabled"
              className="text-[11px] leading-tight text-[var(--pad-text-secondary)]"
            >
              开机自启
            </label>
          </div>

          <div className="flex items-start gap-2 rounded-xl border border-[var(--pad-border)] bg-[var(--pad-surface)] px-2.5 py-2">
            <input
              id="dot_click_through"
              type="checkbox"
              checked={form.dot_click_through}
              onChange={(event) => {
                applyForm({ ...formRef.current, dot_click_through: event.target.checked });
                flushAutoSave();
              }}
              className="mt-0.5 h-3.5 w-3.5 shrink-0 accent-[var(--pad-accent)]"
            />
            <label
              htmlFor="dot_click_through"
              className="text-[11px] leading-tight text-[var(--pad-text-secondary)]"
            >
              圆圈点击穿透：开启后圆圈无法点击，只能用热键{" "}
              {form.global_shortcut.trim() || "Ctrl+Q"} 唤起
            </label>
          </div>
        </div>
      </div>

      <div className="flex shrink-0 items-center justify-between gap-2 border-t border-[var(--pad-border)] px-3.5 py-2">
        <span className="truncate text-[10px] text-[var(--pad-text-tertiary)]">
          {saving ? "保存中…" : saved ? "已自动保存" : "改动会自动保存"}
        </span>
        <div className="flex shrink-0 items-center gap-2">
          <button
            type="button"
            onClick={() => setView("prompt")}
            className="rounded-full border border-[var(--pad-border)] px-3 py-1 text-[12px] font-medium text-[var(--pad-text-secondary)] transition-colors hover:bg-[var(--pad-surface)] hover:text-[var(--pad-text)] focus:outline-none focus-visible:ring-2 focus-visible:ring-[var(--pad-accent-ring)]"
          >
            返回
          </button>
          <button
            type="button"
            onClick={handleSave}
            disabled={saving}
            className="rounded-full bg-[var(--pad-accent)] px-3.5 py-1 text-[12px] font-semibold text-white transition-colors hover:bg-[var(--pad-accent-hover)] focus:outline-none focus-visible:ring-2 focus-visible:ring-[var(--pad-accent-ring)] disabled:cursor-not-allowed disabled:bg-[var(--pad-surface-strong)] disabled:text-[var(--pad-text-tertiary)]"
          >
            立即保存
          </button>
        </div>
      </div>
    </div>
  );
}
