/** Adaptador IPC de áudio (dispositivos e teste de captura). */
import { invokeSafe } from "./client";

/** Dispositivo de áudio exposto pelo core. */
export interface AudioDevice {
  id: string;
  name: string;
  is_default: boolean;
}

/** Lista microfones de entrada. */
export const listMicrophones = (): Promise<AudioDevice[] | undefined> =>
  invokeSafe<AudioDevice[]>("audio_list_devices");

/** Lista monitores de saída (áudio do sistema). */
export const listSystemSources = (): Promise<AudioDevice[] | undefined> =>
  invokeSafe<AudioDevice[]>("system_audio_list_devices");

/** Inicia a captura do microfone (omitido = padrão). */
export const startMicrophone = (
  deviceId?: string | null,
): Promise<void | undefined> =>
  invokeSafe<void>("audio_start", { deviceId: deviceId ?? null });

/** Interrompe a captura do microfone. */
export const stopMicrophone = (): Promise<void | undefined> =>
  invokeSafe<void>("audio_stop");

/** Inicia a captura do áudio do sistema (omitido = monitor padrão). */
export const startSystemAudio = (
  deviceId?: string | null,
): Promise<void | undefined> =>
  invokeSafe<void>("system_audio_start", { deviceId: deviceId ?? null });

/** Interrompe a captura do áudio do sistema. */
export const stopSystemAudio = (): Promise<void | undefined> =>
  invokeSafe<void>("system_audio_stop");

// --- Monitor atrasado do microfone ("ouvir minha voz", task 15) --------------

/** Atraso padrão do monitor (ms), definido pelo core. */
export const monitorDefaultDelay = (): Promise<number | undefined> =>
  invokeSafe<number>("audio_monitor_default_delay");

/** Liga o monitor do microfone (o usuário se ouve com atraso). */
export const startMonitor = (
  delayMs?: number,
): Promise<void | undefined> =>
  invokeSafe<void>("audio_monitor_start", { delayMs: delayMs ?? null });

/** Desliga o monitor do microfone. */
export const stopMonitor = (): Promise<void | undefined> =>
  invokeSafe<void>("audio_monitor_stop");

/** Informa se o monitor está ligado. */
export const monitorIsRunning = (): Promise<boolean | undefined> =>
  invokeSafe<boolean>("audio_monitor_is_running");
