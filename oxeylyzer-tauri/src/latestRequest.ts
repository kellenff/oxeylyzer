export function latestRequest<T>(load: () => Promise<T>, publish: (value: T) => void) {
  let sequence = 0;
  let published = 0;
  return async () => {
    const request = ++sequence;
    let value: T;
    try {
      value = await load();
    } catch (error) {
      if (request > published) throw error;
      return;
    }
    if (request > published) {
      publish(value);
      published = request;
    }
  };
}
