import { useState, useRef, useEffect } from 'react';
import { Send, Loader2, Trash2, MessageSquare, Sparkles, Bot, ChevronRight } from 'lucide-react';
import { useStore } from '../lib/store';
import { ChatMessage } from './ChatMessage';

const SUGGESTED_QUESTIONS = [
  "What's the overall quality of this schematic design?",
  "Are there any potential EMI issues I should address?",
  "What improvements would you suggest for power distribution?",
  "Are there any missing protection components?",
  "Can you review the decoupling capacitor placement?",
  "What thermal considerations should I be aware of?",
];

interface ChatPanelProps {
  onOpenAIAnalysis?: () => void;
}

export function ChatPanel({ onOpenAIAnalysis }: ChatPanelProps) {
  const { 
    project, 
    messages, 
    isChatLoading, 
    sendMessage, 
    clearMessages,
    settings,
    aiAnalysis 
  } = useStore();
  
  const [input, setInput] = useState('');
  const messagesEndRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLTextAreaElement>(null);

  // Auto-scroll to bottom when messages change
  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [messages]);

  // Auto-resize textarea
  useEffect(() => {
    if (inputRef.current) {
      inputRef.current.style.height = 'auto';
      inputRef.current.style.height = `${Math.min(inputRef.current.scrollHeight, 120)}px`;
    }
  }, [input]);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    
    if (!input.trim() || isChatLoading) return;
    
    const message = input.trim();
    setInput('');
    await sendMessage(message);
  };

  const handleSuggestedQuestion = (question: string) => {
    setInput(question);
    inputRef.current?.focus();
  };

  const handleKeyDown = (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      handleSubmit(e);
    }
  };

  // Chat is available whenever a project is loaded; the backend router
  // will choose Claude (with API key) or Ollama based on settings.
  const canChat = !!project;

  return (
    <div className="flex flex-col h-full bg-white dark:bg-gray-800">
      {/* Header */}
      <div className="flex items-center justify-between px-6 py-4 border-b border-gray-200 dark:border-gray-700">
        <div className="flex items-center gap-3">
          <div>
            <h2 className="font-semibold text-gray-900 dark:text-white">AI Assistant</h2>
            <p className="text-xs text-gray-500 dark:text-gray-400">
              Ask questions about your schematic design
            </p>
          </div>
        </div>
        
        {messages.length > 0 && (
          <button
            onClick={clearMessages}
            className="p-2 text-gray-400 hover:text-gray-600 dark:hover:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-700 rounded-lg transition-colors"
            title="Clear chat"
          >
            <Trash2 className="w-4 h-4" />
          </button>
        )}
      </div>

      {/* AI Analysis Summary — clickable to reopen full results */}
      {aiAnalysis && (
        <button
          type="button"
          onClick={onOpenAIAnalysis}
          className="w-full text-left px-6 py-3 bg-purple-50 dark:bg-purple-900/20 border-b border-purple-200 dark:border-purple-800 hover:bg-purple-100 dark:hover:bg-purple-900/30 transition-colors cursor-pointer group"
        >
          <div className="flex items-start gap-2">
            <Sparkles className="w-4 h-4 text-purple-500 mt-0.5 flex-shrink-0" />
            <div className="flex-1 min-w-0">
              <p className="text-sm font-medium text-purple-900 dark:text-purple-200">
                AI Analysis Summary
              </p>
              <p className="text-xs text-purple-700 dark:text-purple-300 mt-1 line-clamp-2">
                {aiAnalysis.summary}
              </p>
            </div>
            <ChevronRight className="w-4 h-4 text-purple-400 dark:text-purple-500 mt-0.5 flex-shrink-0 group-hover:text-purple-600 dark:group-hover:text-purple-300 transition-colors" />
          </div>
        </button>
      )}

      {/* Messages area */}
      <div className="flex-1 overflow-y-auto px-6 py-4 space-y-4">
        {!project ? (
          <div className="flex flex-col items-center justify-center h-full text-center">
            <MessageSquare className="w-12 h-12 text-gray-300 dark:text-gray-600 mb-4" />
            <h3 className="text-lg font-medium text-gray-900 dark:text-white mb-2">
              No Project Loaded
            </h3>
            <p className="text-sm text-gray-500 dark:text-gray-400 max-w-sm">
              Open a KiCAD schematic to start chatting with the AI assistant about your design.
            </p>
          </div>
        ) : messages.length === 0 ? (
            <div className="flex flex-col items-center justify-center h-full">
            <div className="text-center mb-8">
              <div className="w-16 h-16 mx-auto mb-4 rounded-full bg-green-500 flex items-center justify-center">
                <Bot className="w-10 h-10 text-white" />
              </div>
              <h3 className="text-lg font-medium text-gray-900 dark:text-white mb-2">
                How can I help you?
              </h3>
              <p className="text-sm text-gray-500 dark:text-gray-400 max-w-sm">
                Ask me anything about your schematic design. I can help with design reviews, 
                component selection, and best practices.
              </p>
            </div>

            {/* Suggested questions */}
            <div className="w-full max-w-lg">
              <p className="text-xs font-medium text-gray-500 dark:text-gray-400 mb-3 text-center">
                SUGGESTED QUESTIONS
              </p>
              <div className="grid gap-2">
                {SUGGESTED_QUESTIONS.map((question, index) => (
                  <button
                    key={index}
                    onClick={() => handleSuggestedQuestion(question)}
                    disabled={!canChat}
                    className="text-left px-4 py-3 text-sm text-gray-700 dark:text-gray-300 bg-gray-50 dark:bg-gray-700/50 hover:bg-gray-100 dark:hover:bg-gray-700 rounded-lg border border-gray-200 dark:border-gray-600 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
                  >
                    {question}
                  </button>
                ))}
              </div>
            </div>
          </div>
        ) : (
          <>
            {messages.map((message) => (
              <ChatMessage key={message.id} message={message} />
            ))}
            
            {isChatLoading && (
              <div className="flex gap-3">
                <div className="w-8 h-8 rounded-full bg-green-500 flex items-center justify-center flex-shrink-0">
                  <Bot className="w-5 h-5 text-white" />
                </div>
                <div className="bg-gray-100 dark:bg-gray-700 rounded-2xl rounded-bl-md px-4 py-3">
                  <div className="flex items-center gap-1">
                    <span className="w-2 h-2 bg-gray-400 rounded-full animate-bounce" style={{ animationDelay: '0ms' }} />
                    <span className="w-2 h-2 bg-gray-400 rounded-full animate-bounce" style={{ animationDelay: '150ms' }} />
                    <span className="w-2 h-2 bg-gray-400 rounded-full animate-bounce" style={{ animationDelay: '300ms' }} />
                  </div>
                </div>
              </div>
            )}
            
            <div ref={messagesEndRef} />
          </>
        )}
      </div>

      {/* Input area */}
      <div className="px-6 py-4 border-t border-gray-200 dark:border-gray-700">
        <form onSubmit={handleSubmit} className="flex items-end gap-3">
            <div className="flex-1 relative">
              <textarea
                ref={inputRef}
                value={input}
                onChange={(e) => setInput(e.target.value)}
                onKeyDown={handleKeyDown}
                placeholder={project ? "Ask about your schematic..." : "Open a project first..."}
                disabled={!canChat || isChatLoading}
                rows={1}
                className="w-full px-4 py-3 pr-12 text-sm text-gray-900 dark:text-white bg-gray-100 dark:bg-gray-700 border-0 rounded-xl resize-none focus:outline-none focus:ring-2 focus:ring-blue-500 disabled:opacity-50 disabled:cursor-not-allowed placeholder-gray-500 dark:placeholder-gray-400"
              />
            </div>
            
            <button
              type="submit"
              disabled={!canChat || !input.trim() || isChatLoading}
              className="flex-shrink-0 p-3 text-white bg-blue-600 rounded-xl hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 disabled:opacity-50 disabled:cursor-not-allowed transition-colors dark:focus:ring-offset-gray-800"
            >
              {isChatLoading ? (
                <Loader2 className="w-5 h-5 animate-spin" />
              ) : (
                <Send className="w-5 h-5" />
              )}
            </button>
        </form>

        {/* Helper text when Claude key is missing but Ollama could still be used */}
        {!settings.apiKeyConfigured && (
          <div className="mt-2 text-center">
            <p className="text-xs text-gray-500 dark:text-gray-400">
              You can chat using a Claude API key or by selecting the local Ollama provider in Settings.
            </p>
          </div>
        )}
      </div>
    </div>
  );
}
