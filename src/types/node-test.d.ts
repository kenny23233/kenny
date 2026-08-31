// node:test / node:assert ambient declarations
// 项目未安装 @types/node, 但 tauri.test.ts 用 node 内置测试运行器
// 只声明测试用到的最小 API, 够 src/types/tauri.test.ts 编译即可

declare module "node:test" {
  export function test(
    name: string,
    fn: () => void | Promise<void>,
  ): void;
  export function suite(name: string, fn: () => void): void;
}

declare module "node:assert/strict" {
  interface AssertApi {
    equal<T>(actual: T, expected: T, msg?: string): void;
    notEqual<T>(actual: T, expected: T, msg?: string): void;
    deepEqual<T>(actual: T, expected: T, msg?: string): void;
    ok(value: unknown, msg?: string): void;
    fail(msg?: string): never;
  }
  const assert: AssertApi;
  export default assert;
}
