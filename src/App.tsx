import React, { useState } from 'react';
import { invoke } from '@tauri-apps/api/tauri';
import { open, save } from '@tauri-apps/api/dialog';
import { open as openShell } from '@tauri-apps/api/shell';

interface TranslatedFile {
  originalName: string;
  translatedName: string | undefined;
  path: string;
  fullPath: string;
}

function App() {
  const [files, setFiles] = useState<TranslatedFile[]>([]);
  const [isLoading, setIsLoading] = useState(false);

  const processFiles = async (filePaths: string[]) => {
    setIsLoading(true);
    try {
      console.log('Processing files with paths:', filePaths);
      const translatedFiles = await Promise.all(
        filePaths.map(async (fullPath) => {
          try {
            const originalName = fullPath.split(/[/\\]/).pop() || fullPath;
            console.log('Attempting to translate filename:', originalName);
            
            const translatedName = await invoke('translate_filename', { 
              filename: originalName 
            });
            
            console.log('Translation result:', {
              originalName,
              translatedName,
              fullPath
            });
            
            return { 
              originalName, 
              translatedName, 
              path: originalName,
              fullPath
            };
          } catch (fileError) {
            console.error('Error processing individual file:', {
              file: fullPath,
              error: fileError
            });
            // 返回带有错误信息的文件对象，而不是抛出错误
            return {
              originalName: fullPath.split(/[/\\]/).pop() || fullPath,
              translatedName: undefined,
              path: fullPath,
              fullPath
            };
          }
        })
      );

      console.log('All files processed:', translatedFiles);
      setFiles(translatedFiles as TranslatedFile[]);
    } catch (error) {
      console.error('Translation process error:', error);
      if (error instanceof Error) {
        alert(`翻译过程中发生错误：${error.message}`);
      } else {
        alert('翻译过程中发生错误，请重试。');
      }
    } finally {
      setIsLoading(false);
    }
  };

  const handleFileUpload = async () => {
    const selected = await open({
      multiple: true,
      filters: [
        {
          name: '图片文件',
          extensions: ['png', 'jpg', 'jpeg', 'gif', 'webp', 'bmp']
        },
        {
          name: '所有文件',
          extensions: ['*']
        }
      ]
    });
  
    if (Array.isArray(selected) && selected.length > 0) {
      console.log('Selected files:', selected);
      processFiles(selected);
    }
  };

  const handleDownload = async () => {
    try {
      const savePath = await save({
        filters: [{ 
          name: 'ZIP Archive',
          extensions: ['zip']
        }]
      });

      if (savePath) {
        setIsLoading(true);
        const fileList = files.map(file => [
          file.fullPath,
          file.translatedName || file.originalName
        ]);

        await invoke('create_zip_file', {
          files: fileList,
          zipPath: savePath
        });

        setIsLoading(false);
        alert('压缩包保存成功！');
      }
    } catch (error) {
      console.error('Error during download:', error);
      alert('创建压缩包时发生错误，请重试。');
      setIsLoading(false);
    }
  };

  const openLogFile = async () => {
    const tempDir = await invoke('get_temp_dir');
    await openShell(`${tempDir}/translator_app.log`);
  };

  return (
    <div className="container mx-auto p-4 max-w-2xl">
      <h1 className="text-3xl font-bold mb-6 text-center text-gray-800">文件名翻译器</h1>
      <button
        onClick={handleFileUpload}
        className="w-full bg-blue-500 hover:bg-blue-600 text-white font-semibold py-2 px-4 rounded-lg transition-colors mb-6"
      >
        选择文件
      </button>
      
      {isLoading && (
        <div className="text-center mb-6">
          <div className="inline-block animate-spin rounded-full h-8 w-8 border-t-2 border-b-2 border-blue-500"></div>
          <p className="mt-2 text-gray-600">处理中...</p>
        </div>
      )}
      
      {files.length > 0 && (
        <div>
          <h2 className="text-xl font-semibold mb-4 text-gray-700">翻译结果：</h2>
          <ul className="bg-gray-50 rounded-lg p-4 mb-6">
            {files.map((file, index) => (
              <li key={index} className="mb-2 last:mb-0">
                <span className="font-medium text-gray-700">{file.originalName}</span>
                <span className="text-gray-400 mx-2">→</span>
                {typeof file.translatedName === 'string' ? (
                  <span className="text-green-600">{file.translatedName}</span>
                ) : (
                  <span className="text-red-600">翻译失败</span>
                )}
              </li>
            ))}
          </ul>
          <button
            className="w-full bg-green-500 hover:bg-green-600 text-white font-semibold py-2 px-4 rounded-lg transition-colors"
            onClick={handleDownload}
          >
            下载翻译后的文件
          </button>
        </div>
      )}
      
      <button
        onClick={openLogFile}
        className="mt-4 text-sm text-gray-500 hover:text-gray-700"
      >
        查看日志
      </button>
    </div>
  );
}

export default App;

