import React, { useEffect, useRef, useState } from 'react';
import { useAppStore } from '../../lib/store';

interface JarvisVoiceOverlayProps {
  onTranscript: (userText: string, assistantText: string) => void;
  onClose: () => void;
}

export function JarvisVoiceOverlay({ onTranscript, onClose }: JarvisVoiceOverlayProps) {
  const [status, setStatus] = useState('Initializing SUNDAY CORE...');
  const [rms, setRms] = useState(0);
  const [isThinking, setIsThinking] = useState(false);
  
  const audioContextRef = useRef<AudioContext | null>(null);
  const analyserRef = useRef<AnalyserNode | null>(null);
  const mediaStreamRef = useRef<MediaStream | null>(null);
  const mediaRecorderRef = useRef<MediaRecorder | null>(null);
  const chunksRef = useRef<Blob[]>([]);
  
  const voiceLlmEndpoint = "http://127.0.0.1:8082/v1/chat/completions";
  const serverApi = "http://127.0.0.1:8098";

  useEffect(() => {
    startJarvisLoop();
    return () => stopJarvisLoop();
  }, []);

  const startJarvisLoop = async () => {
    try {
      const stream = await navigator.mediaDevices.getUserMedia({ audio: true });
      mediaStreamRef.current = stream;
      
      const audioCtx = new AudioContext();
      audioContextRef.current = audioCtx;
      const source = audioCtx.createMediaStreamSource(stream);
      const analyser = audioCtx.createAnalyser();
      analyser.fftSize = 256;
      source.connect(analyser);
      analyserRef.current = analyser;

      setStatus('Listening...');
      startLevelPolling();
      startRecording();
    } catch (err) {
      setStatus('Microphone Error');
      setTimeout(onClose, 2000);
    }
  };

  const silenceTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const startLevelPolling = () => {
    const poll = () => {
      if (!analyserRef.current) return;
      const data = new Uint8Array(analyserRef.current.frequencyBinCount);
      analyserRef.current.getByteFrequencyData(data);
      let sum = 0;
      for (let i = 0; i < data.length; i++) sum += data[i];
      const avg = sum / data.length;
      const level = avg / 128;
      setRms(level); 

      // Silence detection (VAD)
      if (level < 0.05) { // Threshold for silence
        if (!silenceTimerRef.current && mediaRecorderRef.current?.state === 'recording') {
          silenceTimerRef.current = setTimeout(() => {
            if (mediaRecorderRef.current?.state === 'recording') {
              mediaRecorderRef.current.stop();
            }
          }, 1500); // 1.5 seconds of silence
        }
      } else {
        if (silenceTimerRef.current) {
          clearTimeout(silenceTimerRef.current);
          silenceTimerRef.current = null;
        }
      }

      requestAnimationFrame(poll);
    };
    poll();
  };

  const startRecording = () => {
    if (!mediaStreamRef.current) return;
    chunksRef.current = [];
    const recorder = new MediaRecorder(mediaStreamRef.current);
    recorder.ondataavailable = (e) => {
      if (e.data.size > 0) chunksRef.current.push(e.data);
    };
    recorder.onstop = handleRecordingStop;
    recorder.start();
    mediaRecorderRef.current = recorder;
  };

  const stopManual = () => {
    if (mediaRecorderRef.current?.state === 'recording') {
      mediaRecorderRef.current.stop();
    }
  };

  const handleRecordingStop = async () => {
    if (chunksRef.current.length === 0) return;
    const blob = new Blob(chunksRef.current, { type: 'audio/webm' });
    setIsThinking(true);
    setStatus('Transcribing...');

    try {
      // 1. Transcribe
      const resTrans = await fetch(`${serverApi}/api/transcribe`, {
        method: 'POST',
        headers: { 
          'Content-Type': 'audio/webm', 
          'X-STT-Model': 'pariya47/distill-whisper-th-large-v3-ct2',
          'X-STT-Language': 'th'
        },
        body: blob
      });
      const dataTrans = await resTrans.json();
      const prompt = dataTrans.text;
      
      if (!prompt) {
        setStatus('No speech detected');
        setTimeout(() => { setIsThinking(false); setStatus('Listening...'); startRecording(); }, 1000);
        return;
      }

      // 2. Start Live Turn
      setStatus('Thinking...');
      const resTurn = await fetch(`${serverApi}/api/live-turn`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          route_mode: 'auto',
          messages: [{ role: 'user', content: prompt }],
          voice: 'auto'
        })
      });

      if (!resTurn.body) throw new Error('No response body');
      const reader = resTurn.body.getReader();
      const decoder = new TextDecoder();
      let fullText = '';
      const audioQueue: string[] = [];
      let isPlaying = false;

      const playNext = () => {
        const base64 = audioQueue.shift();
        if (!base64) { isPlaying = false; return; }
        isPlaying = true;
        setStatus('Speaking...');
        const audio = new Audio(`data:audio/mpeg;base64,${base64}`);
        audio.onended = playNext;
        audio.play().catch(playNext);
      };

      while (true) {
        const { value, done } = await reader.read();
        if (done) break;
        const chunk = decoder.decode(value);
        const lines = chunk.split('\n');
        for (const line of lines) {
          if (!line.trim()) continue;
          try {
            const ev = JSON.parse(line);
            if (ev.type === 'text_delta') {
              fullText += ev.delta;
            } else if (ev.type === 'audio') {
              audioQueue.push(ev.audio);
              if (!isPlaying) playNext();
            } else if (ev.type === 'status') {
              setStatus(ev.message);
            }
          } catch {}
        }
      }

      // Wait for audio to finish before resuming listening
      const checkDone = setInterval(() => {
        if (!isPlaying && audioQueue.length === 0) {
          clearInterval(checkDone);
          onTranscript(prompt, fullText); // Sync to main chat history
          
          // RESTART THE LOOP instead of closing
          setIsThinking(false);
          setStatus('Listening...');
          startRecording();
        }
      }, 500);

    } catch (err) {
      setStatus('Error');
      setTimeout(onClose, 2000);
    }
  };

  const stopJarvisLoop = () => {
    mediaRecorderRef.current?.stop();
    mediaStreamRef.current?.getTracks().forEach(t => t.stop());
    audioContextRef.current?.close();
  };

  return (
    <div className="fixed bottom-8 right-8 z-50 flex flex-col items-center p-6 rounded-[2.5rem] transition-all duration-500 animate-in fade-in zoom-in slide-in-from-bottom-10" 
         style={{ 
           background: 'rgba(8, 12, 24, 0.5)', 
           backdropFilter: 'blur(24px) saturate(200%)',
           border: '1px solid rgba(255, 255, 255, 0.08)',
           boxShadow: '0 25px 60px -12px rgba(0, 0, 0, 0.6), inset 0 0 30px rgba(6, 182, 212, 0.03)',
           width: '280px'
         }}>
      
      {/* HUD Header Decor */}
      <div className="absolute top-4 left-1/2 -translate-x-1/2 flex gap-1">
        <div className="w-1.5 h-1.5 rounded-full bg-cyan-500/20" />
        <div className="w-8 h-1.5 rounded-full bg-cyan-500/10" />
        <div className="w-1.5 h-1.5 rounded-full bg-cyan-500/20" />
      </div>

      <div className="relative w-32 h-32 flex items-center justify-center mt-4">
        {/* Organic Energy Rings */}
        <div 
          className="absolute inset-0 rounded-full border border-cyan-500/10"
          style={{ 
            transform: `scale(${1.1 + rms * 0.4})`, 
            opacity: 0.5 + rms,
            transition: 'transform 0.15s cubic-bezier(0.2, 0, 0.2, 1)'
          }}
        />
        <div 
          className="absolute inset-4 rounded-full border border-cyan-400/20"
          style={{ 
            transform: `scale(${1 + rms * 0.2})`, 
            transition: 'transform 0.2s cubic-bezier(0.2, 0, 0.2, 1)'
          }}
        />
        
        {/* Core Fluid Orb */}
        <div className="relative w-16 h-16 rounded-full overflow-hidden flex items-center justify-center">
          <div 
            className={`absolute inset-0 bg-gradient-to-tr from-cyan-600 to-blue-400 blur-[2px] transition-all duration-300 ${isThinking ? 'animate-pulse scale-110' : ''}`}
            style={{ 
              borderRadius: '40% 60% 70% 30% / 40% 50% 60% 50%',
              boxShadow: `0 0 ${20 + rms * 40}px rgba(6, 182, 212, 0.4)`,
              transform: `scale(${1 + rms * 0.6}) rotate(${rms * 45}deg)`,
            }}
          />
          <div className="z-10 w-4 h-4 bg-white rounded-full opacity-40 blur-[1px]" />
        </div>
      </div>

      {/* Status & Indicators */}
      <div className="text-center mt-6 mb-8">
        <p className="font-sans text-[10px] text-cyan-400/60 font-bold tracking-[0.3em] uppercase mb-1">
          SUNDAY CORE
        </p>
        <p className={`font-sans text-sm font-medium tracking-wide ${isThinking ? 'text-white' : 'text-cyan-100'}`}>
          {status}
        </p>
      </div>
      
      {/* Action Controls */}
      <div className="grid grid-cols-2 gap-3 w-full">
        <button 
          onClick={stopManual}
          className="group relative px-4 py-2.5 rounded-2xl transition-all duration-300 overflow-hidden"
          style={{ background: 'rgba(6, 182, 212, 0.1)' }}
        >
          <div className="absolute inset-0 bg-cyan-500 opacity-0 group-hover:opacity-10 transition-opacity" />
          <span className="relative font-sans text-[10px] font-bold text-cyan-400 tracking-wider uppercase">Finish</span>
        </button>
        
        <button 
          onClick={onClose}
          className="group px-4 py-2.5 rounded-2xl transition-all duration-300 border border-white/5 hover:border-white/10 hover:bg-white/5"
        >
          <span className="font-sans text-[10px] font-bold text-white/40 group-hover:text-white/60 tracking-wider uppercase">Dismiss</span>
        </button>
      </div>
    </div>
  );
}
