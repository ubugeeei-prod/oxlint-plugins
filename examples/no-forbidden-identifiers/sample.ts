export const data = {
  error: new Error('example'),
};

export const event = data.error;

export function run(ctx: unknown) {
  return { ctx, event };
}
