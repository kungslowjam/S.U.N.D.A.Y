import { useState, useEffect, useCallback } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import { 
  Bot, Globe, Shield, Play, Pause, CheckCircle2, 
  AlertCircle, ChevronRight, Folder, FileText, 
  Table, Layout, Activity, Terminal
} from 'lucide-react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

interface WorkflowNode {
  id: string;
  label: string;
  type: 'agent' | 'tool' | 'condition' | 'workspace' | 'office-word' | 'office-excel';
  status: 'pending' | 'running' | 'success' | 'error';
  output?: string;
}

export function CommandCenterPage() {
  const [nodes, setNodes] = useState<WorkflowNode[]>([
    { id: '0', label: 'Create Project: Sunday-Dev', type: 'workspace', status: 'success', output: 'Workspace initialized at /workspaces/sunday-dev' },
    { id: '1', label: 'Analyze Data Source', type: 'agent', status: 'success', output: 'Found 12 relevant sales records.' },
    { id: '2', label: 'Extract Excel Data', type: 'office-excel', status: 'running' },
    { id: '3', label: 'Generate Word Report', type: 'office-word', status: 'pending' },
  ]);

  const [browserUrl, setBrowserUrl] = useState('https://github.com/different-ai/openwork');
  const [showApproval, setShowApproval] = useState(false);
  const [isExecuting, setIsExecuting] = useState(false);
  const [activeProject, setActiveProject] = useState('Sunday-Dev');

  useEffect(() => {
    const unlisten = listen('workflow-finished', (event) => {
      console.log('Workflow finished:', event.payload);
      setIsExecuting(false);
      setNodes(prev => prev.map(n => ({ ...n, status: 'success' })));
    });
    return () => { unlisten.then(f => f()); };
  }, []);

  const executeMission = async () => {
    setIsExecuting(true);
    setNodes(prev => prev.map((n, i) => i === 2 ? { ...n, status: 'running' } : n));
    
    try {
      const graphJson = JSON.stringify({
        nodes: nodes.map(n => ({ id: n.id, label: n.label, kind: n.type })),
        edges: []
      });
      await invoke('run_workflow', { graphJson, initialInput: 'Process excel data and create report' });
    } catch (e) {
      console.error('Workflow failed:', e);
      setIsExecuting(false);
    }
  };

  return (
    <div className="flex flex-col h-full bg-[#0a0a0c] text-slate-200 overflow-hidden font-sans">
      {/* Top Navigation / Status Bar */}
      <header className="flex items-center justify-between px-8 py-5 border-b border-white/5 bg-black/40 backdrop-blur-xl z-50">
        <div className="flex items-center gap-6">
          <div className="relative">
            <div className="absolute -inset-1 bg-primary/20 blur-sm rounded-lg animate-pulse" />
            <div className="relative p-2.5 bg-primary/10 rounded-xl border border-primary/20">
              <Bot className="w-6 h-6 text-primary" />
            </div>
          </div>
          <div className="flex flex-col">
            <div className="flex items-center gap-2">
              <h1 className="text-xl font-black tracking-tighter text-white uppercase">SUNDAY</h1>
              <span className="px-2 py-0.5 bg-primary/20 text-primary text-[10px] font-bold rounded border border-primary/30 uppercase tracking-widest">Command Center</span>
            </div>
            <div className="flex items-center gap-2 mt-1 text-[10px] font-medium text-slate-500">
              <Activity className="w-3 h-3 text-green-500" />
              <span>Native Engine Active</span>
              <span className="w-1 h-1 rounded-full bg-slate-700" />
              <Folder className="w-3 h-3 text-orange-400" />
              <span className="text-slate-300">Project: {activeProject}</span>
            </div>
          </div>
        </div>

        <div className="flex items-center gap-4">
          <div className="flex -space-x-2 mr-4">
            {[1, 2, 3].map(i => (
              <div key={i} className="w-8 h-8 rounded-full border-2 border-[#0a0a0c] bg-slate-800 flex items-center justify-center text-[10px] font-bold overflow-hidden ring-1 ring-white/5">
                <img src={`https://api.dicebear.com/7.x/bottts/svg?seed=agent${i}`} alt="agent" />
              </div>
            ))}
          </div>
          <button 
            onClick={executeMission}
            disabled={isExecuting}
            className={`
              flex items-center gap-2 px-6 py-2.5 rounded-full font-bold text-sm transition-all duration-300
              ${isExecuting 
                ? 'bg-primary/10 text-primary border border-primary/20 cursor-wait' 
                : 'bg-primary text-primary-foreground hover:shadow-[0_0_20px_rgba(var(--primary),0.3)] hover:scale-105 active:scale-95'}
            `}
          >
            {isExecuting ? (
              <><Pause className="w-4 h-4 fill-current" /> EXECUTING MISSION</>
            ) : (
              <><Play className="w-4 h-4 fill-current" /> INITIATE MISSION</>
            )}
          </button>
        </div>
      </header>

      {/* Main Content Split View */}
      <div className="flex-1 flex overflow-hidden">
        {/* Left: Visual Thinking Graph */}
        <div className="flex-[3.5] p-8 bg-[url('https://grainy-gradients.vercel.app/noise.svg')] relative overflow-hidden border-r border-white/5">
          {/* Animated Background Gradients */}
          <div className="absolute top-0 -left-1/4 w-[50%] h-[50%] bg-primary/10 blur-[120px] rounded-full pointer-events-none" />
          <div className="absolute bottom-0 -right-1/4 w-[50%] h-[50%] bg-blue-500/10 blur-[120px] rounded-full pointer-events-none" />
          
          <div className="relative z-10 h-full flex flex-col">
            <div className="flex items-center justify-between mb-12">
              <div className="flex items-center gap-3">
                <div className="p-2 bg-white/5 rounded-lg border border-white/10">
                  <Terminal className="w-4 h-4 text-primary" />
                </div>
                <h2 className="text-xs font-bold uppercase tracking-[0.2em] text-slate-400">
                  Visual Reasoning Pipeline
                </h2>
              </div>
            </div>

            <div className="flex-1 flex flex-col items-center justify-start gap-16 py-10 overflow-y-auto custom-scrollbar">
              {nodes.map((node, index) => (
                <div key={node.id} className="relative flex flex-col items-center w-full">
                  <motion.div
                    initial={{ opacity: 0, y: 20, scale: 0.95 }}
                    animate={{ opacity: 1, y: 0, scale: 1 }}
                    transition={{ delay: index * 0.1, type: 'spring', stiffness: 100 }}
                    className={`
                      w-[400px] p-5 rounded-2xl border-2 backdrop-blur-2xl transition-all duration-700 relative group
                      ${node.status === 'running' 
                        ? 'border-primary bg-primary/5 shadow-[0_0_40px_rgba(var(--primary),0.15)]' 
                        : 'border-white/5 bg-white/[0.03] hover:border-white/10'}
                      ${node.status === 'success' ? 'border-green-500/30 bg-green-500/[0.02]' : ''}
                    `}
                  >
                    {/* Status Glow */}
                    {node.status === 'running' && (
                      <div className="absolute -inset-0.5 bg-gradient-to-r from-primary/50 to-blue-500/50 rounded-2xl opacity-20 blur-sm animate-pulse" />
                    )}

                    <div className="flex items-start justify-between relative z-10">
                      <div className="flex items-center gap-4">
                        <div className={`
                          p-3 rounded-xl border
                          ${node.status === 'running' ? 'bg-primary/20 border-primary/30' : 'bg-white/5 border-white/10'}
                          ${node.status === 'success' ? 'bg-green-500/20 border-green-500/30' : ''}
                        `}>
                          {node.type === 'agent' && <Bot className="w-5 h-5 text-blue-400" />}
                          {node.type === 'tool' && <Globe className="w-5 h-5 text-purple-400" />}
                          {node.type === 'workspace' && <Folder className="w-5 h-5 text-orange-400" />}
                          {node.type === 'office-excel' && <Table className="w-5 h-5 text-green-400" />}
                          {node.type === 'office-word' && <FileText className="w-5 h-5 text-blue-400" />}
                          {node.type === 'condition' && <Shield className="w-5 h-5 text-orange-400" />}
                        </div>
                        <div>
                          <div className="text-[10px] font-bold text-slate-500 uppercase tracking-widest mb-1">{node.type}</div>
                          <span className="text-sm font-black tracking-tight text-white group-hover:text-primary transition-colors">{node.label}</span>
                        </div>
                      </div>
                      <div className="pt-1">
                        {node.status === 'success' && <div className="w-6 h-6 bg-green-500/20 rounded-full flex items-center justify-center"><CheckCircle2 className="w-4 h-4 text-green-500" /></div>}
                        {node.status === 'running' && <motion.div animate={{ rotate: 360 }} transition={{ repeat: Infinity, duration: 2, ease: "linear" }}><Play className="w-4 h-4 text-primary fill-current" /></motion.div>}
                      </div>
                    </div>
                    {node.output && (
                      <motion.div 
                        initial={{ opacity: 0 }}
                        animate={{ opacity: 1 }}
                        className="mt-4 p-3 bg-black/40 rounded-lg border border-white/5 font-mono text-[10px] text-slate-400 leading-relaxed"
                      >
                        <span className="text-primary mr-2">❯</span> {node.output}
                      </motion.div>
                    )}
                  </motion.div>
                  
                  {index < nodes.length - 1 && (
                    <div className="h-16 w-0.5 relative my-2">
                      <div className="absolute inset-0 bg-white/5 rounded-full" />
                      <motion.div 
                        initial={{ height: 0 }}
                        animate={{ height: '100%' }}
                        className={`absolute top-0 left-0 w-full rounded-full ${node.status === 'success' ? 'bg-green-500/50' : 'bg-primary/50 opacity-20'}`}
                        transition={{ duration: 1 }}
                      />
                      <ChevronRight className="absolute -bottom-4 -left-2 w-4 h-4 text-white/10 rotate-90" />
                    </div>
                  )}
                </div>
              ))}
            </div>
          </div>
        </div>

        {/* Right: Browser & Meta Side */}
        <div className="flex-[2.5] flex flex-col bg-black/20 backdrop-blur-md">
          {/* Browser View */}
          <div className="flex-1 flex flex-col border-b border-white/5">
            <div className="px-6 py-3 bg-white/[0.02] flex items-center gap-6">
              <div className="flex items-center gap-3 flex-1 bg-black/40 px-4 py-2 rounded-xl border border-white/10 shadow-inner">
                <Globe className="w-3.5 h-3.5 text-slate-500" />
                <span className="text-xs font-medium text-slate-300 truncate">{browserUrl}</span>
              </div>
              <div className="flex items-center gap-2 px-3 py-1 bg-green-500/10 border border-green-500/20 rounded-lg">
                <div className="w-1.5 h-1.5 rounded-full bg-green-500 animate-pulse" />
                <span className="text-[10px] font-bold text-green-500 uppercase tracking-wider">AX-Tree Live</span>
              </div>
            </div>
            <div className="flex-1 bg-[#f8fafc] relative group overflow-hidden m-4 rounded-2xl border border-white/10 shadow-2xl">
              <iframe src={browserUrl} className="w-full h-full border-none opacity-90 scale-100 group-hover:scale-[1.01] transition-transform duration-700" />
              <div className="absolute inset-0 pointer-events-none border-2 border-primary/0 group-hover:border-primary/20 transition-all duration-500" />
              
              {/* AX-Tree Visual Overlay Mockup */}
              <motion.div 
                animate={{ scale: [1, 1.05, 1], opacity: [0.8, 1, 0.8] }}
                transition={{ duration: 4, repeat: Infinity }}
                className="absolute top-10 left-10 p-2.5 bg-blue-500/10 backdrop-blur-md border border-blue-500/30 rounded-lg text-[9px] text-blue-400 font-bold font-mono shadow-lg"
              >
                button#submit-repo [Role: button]
              </motion.div>
              <div className="absolute top-32 left-20 p-2.5 bg-purple-500/10 backdrop-blur-md border border-purple-500/30 rounded-lg text-[9px] text-purple-400 font-bold font-mono shadow-lg">
                input#search [Role: searchbox]
              </div>
            </div>
          </div>

          {/* Action Logs / Approval Mini-panel */}
          <div className="h-80 p-6 flex flex-col bg-black/40 border-t border-white/5">
            <div className="flex items-center justify-between mb-4">
              <h3 className="text-[10px] font-bold uppercase tracking-[0.2em] text-slate-500">Real-time Telemetry</h3>
              <div className="flex items-center gap-1.5">
                <span className="w-1.5 h-1.5 rounded-full bg-primary" />
                <span className="text-[9px] font-bold text-slate-400 uppercase tracking-widest">Streaming</span>
              </div>
            </div>
            <div className="flex-1 overflow-y-auto space-y-3 font-mono text-[10px] custom-scrollbar pr-2">
              <div className="flex items-center gap-3 text-blue-400/80 bg-blue-500/5 p-2 rounded-lg border border-blue-500/10">
                <span className="text-blue-500 font-bold">INFO</span>
                <span>Initializing Project: Sunday-Dev at /workspaces/...</span>
              </div>
              <div className="flex items-center gap-3 text-purple-400/80 p-2">
                <span className="text-purple-500 font-bold">TOOL</span>
                <span>excel_read(path: "sales_q1.xlsx", range: "A1:G20")</span>
              </div>
              <div className="flex items-center gap-3 text-green-400/80 p-2">
                <span className="text-green-500 font-bold">DONE</span>
                <span>Data extraction complete. 42 entities found.</span>
              </div>
              <div className="flex items-center gap-3 text-orange-400 animate-pulse bg-orange-500/5 p-2 rounded-lg border border-orange-500/10">
                <span className="text-orange-500 font-bold">WAIT</span>
                <span>Awaiting signature for FileSystem write...</span>
              </div>
            </div>
            <button 
              onClick={() => setShowApproval(true)}
              className="mt-6 w-full py-3.5 bg-white/5 hover:bg-white/10 text-white border border-white/10 rounded-xl font-bold text-xs tracking-widest transition-all hover:scale-[1.02] active:scale-[0.98] shadow-lg shadow-black/20"
            >
              REVIEW PENDING ACTIONS
            </button>
          </div>
        </div>
      </div>

      {/* Glassy Approval Modal */}
      <AnimatePresence>
        {showApproval && (
          <div className="fixed inset-0 z-[100] flex items-center justify-center p-6 bg-black/60 backdrop-blur-md">
            <motion.div
              initial={{ scale: 0.9, opacity: 0, y: 20 }}
              animate={{ scale: 1, opacity: 1, y: 0 }}
              exit={{ scale: 0.9, opacity: 0, y: 20 }}
              className="w-full max-w-lg bg-[#121214] border border-white/10 shadow-[0_25px_50px_-12px_rgba(0,0,0,0.5)] rounded-[32px] overflow-hidden"
            >
              <div className="p-8">
                <div className="flex items-center gap-4 mb-8">
                  <div className="p-3 bg-orange-500/10 rounded-2xl text-orange-500 border border-orange-500/20">
                    <Shield className="w-8 h-8" />
                  </div>
                  <div>
                    <h3 className="text-xl font-black text-white tracking-tight uppercase">Action Authorization</h3>
                    <p className="text-xs text-slate-500 font-medium">Privileged write access requested for /workspaces/sunday-dev</p>
                  </div>
                </div>
                
                <div className="p-6 bg-black/60 rounded-2xl font-mono text-sm border border-white/5 mb-8 relative group overflow-hidden">
                  <div className="absolute top-0 left-0 w-1 h-full bg-primary" />
                  <div className="text-slate-500 mb-2 flex items-center gap-2">
                    <Layout className="w-3 h-3" />
                    <span>SYSTEM_COMMAND</span>
                  </div>
                  <span className="text-primary mr-2">$</span> 
                  <span className="text-slate-200">word_write --text "Sales Summary 2024" --path report.docx</span>
                </div>

                <div className="flex gap-4">
                  <button 
                    onClick={() => setShowApproval(false)}
                    className="flex-1 py-4 bg-white/5 hover:bg-white/10 text-slate-400 rounded-2xl font-black text-xs uppercase tracking-widest transition-all"
                  >
                    Discard
                  </button>
                  <button 
                    onClick={() => setShowApproval(false)}
                    className="flex-1 py-4 bg-primary text-primary-foreground hover:shadow-[0_0_20px_rgba(var(--primary),0.3)] rounded-2xl font-black text-xs uppercase tracking-widest transition-all hover:scale-105 active:scale-95"
                  >
                    Authorize & Commit
                  </button>
                </div>
              </div>
              <div className="px-8 py-4 bg-white/[0.02] border-t border-white/5 flex items-center gap-3">
                <AlertCircle className="w-4 h-4 text-orange-500/60" />
                <span className="text-[10px] font-bold text-slate-500 uppercase tracking-widest">This will synchronize changes to the Sunday-Dev workspace.</span>
              </div>
            </motion.div>
          </div>
        )}
      </AnimatePresence>

      <style dangerouslySetInnerHTML={{ __html: `
        .custom-scrollbar::-webkit-scrollbar {
          width: 4px;
        }
        .custom-scrollbar::-webkit-scrollbar-track {
          background: transparent;
        }
        .custom-scrollbar::-webkit-scrollbar-thumb {
          background: rgba(255, 255, 255, 0.1);
          border-radius: 10px;
        }
        .custom-scrollbar::-webkit-scrollbar-thumb:hover {
          background: rgba(var(--primary), 0.5);
        }
        :root {
          --primary: 96, 165, 250; /* blue-400 */
        }
      `}} />
    </div>
  );
}
