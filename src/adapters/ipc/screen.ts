/** Adaptador IPC da captura de tela (BR-055). */
import { invokeSafe } from "./client";

/** Imagem capturada (base64, sem o prefixo `data:`). */
export interface CapturedScreen {
  mime: string;
  data: string;
  width: number;
  height: number;
}

/** Captura a tela (as janelas do app são escondidas durante a captura). */
export const screenCapture = (): Promise<CapturedScreen | undefined> =>
  invokeSafe<CapturedScreen>("screen_capture");

/** Print capturado e ainda **não enviado** (anexo da pergunta). */
export const capturePending = (): Promise<CapturedScreen | null | undefined> =>
  invokeSafe<CapturedScreen | null>("capture_pending");

/** Descarta o print pendente (fecha a prévia). */
export const captureClear = (): Promise<void | undefined> =>
  invokeSafe<void>("capture_clear");

/** Data URL de uma imagem capturada (para a miniatura). */
export const capturedDataUrl = (image: CapturedScreen): string =>
  `data:${image.mime};base64,${image.data}`;
