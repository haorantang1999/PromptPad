import { beforeEach, describe, expect, it, vi } from "vitest";

const invokeMock = vi.fn();

vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => invokeMock(...args),
}));

const { AppError, ErrorCode, call } = await import("./client");

describe("call", () => {
  beforeEach(() => {
    invokeMock.mockReset();
  });

  it("原样透传后端的成功结果", async () => {
    invokeMock.mockResolvedValue({ mode: "expanded" });
    await expect(call<{ mode: string }>("toggle_panel")).resolves.toEqual({ mode: "expanded" });
    expect(invokeMock).toHaveBeenCalledWith("toggle_panel", undefined);
  });

  it("把后端的结构化错误归一化为 AppError", async () => {
    invokeMock.mockRejectedValue({
      code: "INVALID_INPUT",
      message: "提示词不能为空",
      details: "field=prompt",
    });

    await expect(call("optimize_prompt")).rejects.toBeInstanceOf(AppError);
  });

  it("字符串错误回落到 UNKNOWN 错误码", async () => {
    invokeMock.mockRejectedValue("boom");

    try {
      await call("optimize_prompt");
      expect.unreachable("应当抛出错误");
    } catch (err) {
      expect(err).toBeInstanceOf(AppError);
      // 动态 import 解构后拿到的是构造函数，实例类型要用 InstanceType 取。
      expect((err as InstanceType<typeof AppError>).code).toBe(ErrorCode.INTERNAL_ERROR);
      expect((err as InstanceType<typeof AppError>).message).toBe("boom");
    }
  });

  it("非结构化异常也归一化为 AppError", async () => {
    invokeMock.mockRejectedValue(new Error("network down"));

    try {
      await call("optimize_prompt");
      expect.unreachable("应当抛出错误");
    } catch (err) {
      expect(err).toBeInstanceOf(AppError);
      expect((err as InstanceType<typeof AppError>).code).toBe(ErrorCode.INTERNAL_ERROR);
    }
  });
});

describe("AppError", () => {
  it("模型超时属于可恢复错误", () => {
    const err = new AppError(ErrorCode.MODEL_UNAVAILABLE, "调用模型服务失败");
    expect(err.recoverable).toBe(true);
  });

  it("记忆预算超限属于不可恢复错误", () => {
    const err = new AppError(ErrorCode.MEMORY_BUDGET_EXCEEDED, "超出 800 token 上限");
    expect(err.recoverable).toBe(false);
  });
});
