import { useState, useCallback, useRef, useEffect } from 'react';
import { transcribeAudio, fetchSpeechHealth } from '../lib/api';

export type SpeechState = 'idle' | 'recording' | 'transcribing';

type BrowserRecognition = {
  lang: string;
  interimResults: boolean;
  continuous: boolean;
  onresult: ((event: any) => void) | null;
  onerror: ((event: any) => void) | null;
  onend: (() => void) | null;
  start: () => void;
  stop: () => void;
};

function createBrowserRecognition(): BrowserRecognition | null {
  const w = window as any;
  const Ctor = w.SpeechRecognition || w.webkitSpeechRecognition;
  if (!Ctor) return null;
  const recognition = new Ctor() as BrowserRecognition;
  recognition.lang = navigator.language || 'th-TH';
  recognition.interimResults = false;
  recognition.continuous = false;
  return recognition;
}

export function useSpeech() {
  const [state, setState] = useState<SpeechState>('idle');
  const [error, setError] = useState<string | null>(null);
  const [available, setAvailable] = useState(false);
  const mediaRecorderRef = useRef<MediaRecorder | null>(null);
  const recognitionRef = useRef<BrowserRecognition | null>(null);
  const browserTextRef = useRef<string>('');
  const usingBrowserRecognitionRef = useRef(false);
  const chunksRef = useRef<Blob[]>([]);
  const streamRef = useRef<MediaStream | null>(null);

  // Check if speech backend is available on mount
  useEffect(() => {
    fetchSpeechHealth()
      .then((health) => setAvailable(health.available || !!createBrowserRecognition()))
      .catch(() => setAvailable(!!createBrowserRecognition()));
  }, []);

  const startRecording = useCallback(async (): Promise<void> => {
    setError(null);

    if (!navigator.mediaDevices?.getUserMedia) {
      setError('Microphone not supported in this browser');
      return;
    }

    try {
      const recognition = createBrowserRecognition();
      if (recognition) {
        browserTextRef.current = '';
        usingBrowserRecognitionRef.current = true;
        recognition.onresult = (event: any) => {
          const results = Array.from(event.results || []);
          browserTextRef.current = results
            .map((result: any) => result?.[0]?.transcript || '')
            .join(' ')
            .trim();
        };
        recognition.onerror = () => {
          setError('Speech recognition failed');
          setState('idle');
        };
        recognition.onend = () => {
          if (state === 'recording') setState('idle');
        };
        recognitionRef.current = recognition;
        recognition.start();
        setState('recording');
        return;
      }

      const stream = await navigator.mediaDevices.getUserMedia({ audio: true });
      streamRef.current = stream;

      const recorder = new MediaRecorder(stream);
      chunksRef.current = [];

      recorder.ondataavailable = (e) => {
        if (e.data.size > 0) chunksRef.current.push(e.data);
      };

      recorder.start();
      mediaRecorderRef.current = recorder;
      setState('recording');
    } catch (err) {
      setError('Microphone access denied');
      setState('idle');
    }
  }, []);

  const stopRecording = useCallback(async (): Promise<string> => {
    return new Promise((resolve, reject) => {
      if (usingBrowserRecognitionRef.current) {
        const recognition = recognitionRef.current;
        if (!recognition) {
          usingBrowserRecognitionRef.current = false;
          setState('idle');
          resolve('');
          return;
        }
        recognition.onend = () => {
          const text = browserTextRef.current;
          browserTextRef.current = '';
          recognitionRef.current = null;
          usingBrowserRecognitionRef.current = false;
          setState('idle');
          resolve(text);
        };
        recognition.onerror = (event: any) => {
          recognitionRef.current = null;
          usingBrowserRecognitionRef.current = false;
          setState('idle');
          reject(new Error(event?.error || 'Speech recognition failed'));
        };
        recognition.stop();
        return;
      }

      const recorder = mediaRecorderRef.current;
      if (!recorder || recorder.state !== 'recording') {
        reject(new Error('Not recording'));
        return;
      }

      recorder.onstop = async () => {
        setState('transcribing');

        // Stop all audio tracks
        streamRef.current?.getTracks().forEach((track) => track.stop());
        streamRef.current = null;

        const blob = new Blob(chunksRef.current, { type: recorder.mimeType || 'audio/webm' });
        chunksRef.current = [];

        try {
          const result = await transcribeAudio(blob);
          setState('idle');
          resolve(result.text);
        } catch (err) {
          setState('idle');
          const msg = err instanceof Error ? err.message : 'Transcription failed';
          setError(msg);
          reject(err);
        }
      };

      recorder.stop();
    });
  }, []);

  return {
    state,
    error,
    available,
    startRecording,
    stopRecording,
    isRecording: state === 'recording',
    isTranscribing: state === 'transcribing',
  };
}
