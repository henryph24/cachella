export type Operation = {
  requestId?: string;
  cancelRequested: boolean;
  cancellationAccepted?: boolean;
  cancelPromise?: Promise<boolean>;
};

type Invoke = <T>(command: string, args: { requestId: string }) => Promise<T>;

export function cancelOperation(
  operation: Operation,
  invoke: Invoke,
  onRejected: () => void,
  onError: (error: unknown) => void,
): Promise<boolean | undefined> | undefined {
  if (operation.cancellationAccepted !== undefined) return Promise.resolve(operation.cancellationAccepted);
  operation.cancelRequested = true;
  if (!operation.requestId) return;
  if (operation.cancelPromise) return operation.cancelPromise;
  const requestId = operation.requestId;
  operation.cancelPromise = invoke<boolean>('cancel_operation', { requestId }).then(accepted => {
    operation.cancellationAccepted = accepted;
    if (!accepted) {
      operation.cancelRequested = false;
      onRejected();
    }
    return accepted;
  }).catch(error => {
    operation.cancelRequested = false;
    operation.cancelPromise = undefined;
    onError(error);
    throw error;
  });
  return operation.cancelPromise;
}
