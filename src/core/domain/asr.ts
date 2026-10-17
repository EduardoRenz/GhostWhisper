/** Fase de preparação do motor de transcrição (espelha o Rust). */
export type AsrPhase = "preparing" | "ready" | "failed";

/** Evento `asr_status` emitido pelo backend. */
export interface AsrStatusEvent {
  phase: AsrPhase;
  model: string;
  message: string;
}