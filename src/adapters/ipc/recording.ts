/** Adaptador IPC da gravação da sessão (BR-065). */
import { invokeSafe } from "./client";
import type {
  RecordingCapabilities,
  RecordingState,
  RecordingTarget,
} from "../../core/domain/recording";

/** O ambiente consegue gravar? */
export const recordingCapabilities = (): Promise<
  RecordingCapabilities | undefined
> => invokeSafe<RecordingCapabilities>("recording_capabilities");

/** Inicia a gravação da sessão ativa (alvo da configuração). */
export const recordingStart = (): Promise<void | undefined> =>
  invokeSafe<void>("recording_start");

/** Inicia a gravação num alvo escolhido no seletor. */
export const recordingStartWith = (
  target: RecordingTarget,
): Promise<void | undefined> =>
  invokeSafe<void>("recording_start", { target });

/** Troca o alvo no meio da gravação. */
export const recordingSwitchTarget = (
  target: RecordingTarget,
): Promise<void | undefined> =>
  invokeSafe<void>("recording_switch_target", { target });

/** Alvos do seletor: tela inteira + janelas. */
export const recordingTargets = (): Promise<RecordingTarget[] | undefined> =>
  invokeSafe<RecordingTarget[]>("recording_targets");

/** Abre o seletor de janelas. */
export const recordingPickerOpen = (): Promise<void | undefined> =>
  invokeSafe<void>("recording_picker_open");

/** Fecha o seletor de janelas. */
export const recordingPickerClose = (): Promise<void | undefined> =>
  invokeSafe<void>("recording_picker_close");

/** Pausa a gravação (a sessão continua). */
export const recordingPause = (): Promise<void | undefined> =>
  invokeSafe<void>("recording_pause");

/** Retoma a gravação pausada. */
export const recordingResume = (): Promise<void | undefined> =>
  invokeSafe<void>("recording_resume");

/** Encerra a gravação e agenda a finalização. */
export const recordingStop = (): Promise<void | undefined> =>
  invokeSafe<void>("recording_stop");

/** Estado atual (seed do evento). */
export const recordingState = (): Promise<RecordingState | undefined> =>
  invokeSafe<RecordingState>("recording_state");

/** Apaga arquivo e segmentos; a sessão permanece. */
export const recordingDiscard = (
  sessionId: string,
): Promise<void | undefined> =>
  invokeSafe<void>("recording_discard", { sessionId });
