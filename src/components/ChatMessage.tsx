import { User, Bot } from 'lucide-react';
import type { ChatMessage as ChatMessageType } from '../types';

interface ChatMessageProps {
  message: ChatMessageType;
}

export function ChatMessage({ message }: ChatMessageProps) {
  const isUser = message.role === 'user';

  return (
    <div className={`flex gap-3 ${isUser ? 'justify-end' : 'justify-start'}`}>
      {!isUser && (
        <div className="flex-shrink-0 w-8 h-8 rounded-full bg-green-500 flex items-center justify-center">
          <Bot className="w-5 h-5 text-white" />
        </div>
      )}
      
      <div
        className={`max-w-[80%] rounded-2xl px-4 py-2.5 ${
          isUser
            ? 'bg-blue-600 text-white rounded-br-md'
            : 'bg-gray-100 dark:bg-gray-700 text-gray-900 dark:text-white rounded-bl-md'
        }`}
      >
        <div className="text-sm whitespace-pre-wrap break-words">
          {formatMessage(message.content)}
        </div>
        
        <div className={`text-xs mt-1 ${isUser ? 'text-blue-200' : 'text-gray-400'}`}>
          {formatTime(message.timestamp)}
        </div>
      </div>

      {isUser && (
        <div className="flex-shrink-0 w-8 h-8 rounded-full bg-gray-200 dark:bg-gray-600 flex items-center justify-center">
          <User className="w-5 h-5 text-gray-600 dark:text-gray-300" />
        </div>
      )}
    </div>
  );
}

function formatTime(date: Date): string {
  return new Date(date).toLocaleTimeString([], { 
    hour: '2-digit', 
    minute: '2-digit' 
  });
}

function formatMessage(content: string): React.ReactNode {
  // Simple markdown-like formatting
  // Bold: **text**
  // Code: `code`
  // Code block: ```code```
  
  const parts: React.ReactNode[] = [];
  
  // Handle code blocks first
  const codeBlockRegex = /```([\s\S]*?)```/g;
  let match;
  let lastIndex = 0;
  
  while ((match = codeBlockRegex.exec(content)) !== null) {
    if (match.index > lastIndex) {
      parts.push(
        <span key={`text-${lastIndex}`}>
          {formatInlineContent(content.slice(lastIndex, match.index))}
        </span>
      );
    }
    
    parts.push(
      <pre
        key={`code-${match.index}`}
        className="my-2 p-3 bg-gray-800 dark:bg-gray-900 text-gray-100 rounded-lg overflow-x-auto text-xs font-mono"
      >
        <code>{match[1].trim()}</code>
      </pre>
    );
    
    lastIndex = match.index + match[0].length;
  }
  
  if (lastIndex < content.length) {
    parts.push(
      <span key={`text-${lastIndex}`}>
        {formatInlineContent(content.slice(lastIndex))}
      </span>
    );
  }
  
  return parts.length > 0 ? parts : formatInlineContent(content);
}

function formatInlineContent(text: string): React.ReactNode {
  // Handle inline code and bold
  const parts: React.ReactNode[] = [];
  const regex = /(`[^`]+`)|(\*\*[^*]+\*\*)/g;
  let lastIndex = 0;
  let match;
  
  while ((match = regex.exec(text)) !== null) {
    if (match.index > lastIndex) {
      parts.push(text.slice(lastIndex, match.index));
    }
    
    if (match[1]) {
      // Inline code
      parts.push(
        <code
          key={match.index}
          className="px-1.5 py-0.5 bg-gray-200 dark:bg-gray-600 rounded text-xs font-mono"
        >
          {match[1].slice(1, -1)}
        </code>
      );
    } else if (match[2]) {
      // Bold
      parts.push(
        <strong key={match.index}>
          {match[2].slice(2, -2)}
        </strong>
      );
    }
    
    lastIndex = match.index + match[0].length;
  }
  
  if (lastIndex < text.length) {
    parts.push(text.slice(lastIndex));
  }
  
  return parts.length > 0 ? parts : text;
}
