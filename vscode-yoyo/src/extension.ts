import * as vscode from 'vscode';
import * as fs from 'fs';
import * as path from 'path';

interface ToolEvent {
    ts: string;
    tool: string;
    path: string;
    old_text: string;
    new_text: string;
}

let watcher: fs.FSWatcher | undefined;
let lastFileSize = 0;
let watching = false;
let statusBarItem: vscode.StatusBarItem;
let responsePanel: vscode.WebviewPanel | undefined;
let responseWatcher: fs.FSWatcher | undefined;
let responsePoller: NodeJS.Timeout | undefined;
let lastResponseMtime = 0;
let lastResponseContent = '';

function getYoyoRoot(): string | undefined {
    const config = vscode.workspace.getConfiguration('yoyo');
    const configuredPath = config.get<string>('projectPath', '');
    if (configuredPath && fs.existsSync(configuredPath)) {
        return configuredPath;
    }
    return vscode.workspace.workspaceFolders?.[0]?.uri.fsPath;
}

export function activate(context: vscode.ExtensionContext) {
    const workspaceRoot = getYoyoRoot();
    if (!workspaceRoot) return;

    statusBarItem = vscode.window.createStatusBarItem(vscode.StatusBarAlignment.Left, 100);
    statusBarItem.command = 'yoyo.toggleWatch';
    context.subscriptions.push(statusBarItem);

    // Register commands
    context.subscriptions.push(
        vscode.commands.registerCommand('yoyo.showDiff', () => showLastDiff(workspaceRoot)),
        vscode.commands.registerCommand('yoyo.toggleWatch', () => {
            if (watching) {
                stopWatching();
            } else {
                startWatching(workspaceRoot);
            }
        }),
        vscode.commands.registerCommand('yoyo.showResponse', () => {
            openResponsePanel(context, workspaceRoot);
        })
    );

    // Auto-start if .yoyo directory exists
    const yoyoDir = path.join(workspaceRoot, '.yoyo');
    if (fs.existsSync(yoyoDir)) {
        const autoShow = vscode.workspace.getConfiguration('yoyo').get('autoShowDiff', true);
        if (autoShow) {
            startWatching(workspaceRoot);
        }
    }

    updateStatusBar();
}

// ─── Response webview panel ─────────────────────────────────────────

function openResponsePanel(context: vscode.ExtensionContext, workspaceRoot: string) {
    if (responsePanel) {
        responsePanel.reveal(vscode.ViewColumn.Beside);
        return;
    }

    responsePanel = vscode.window.createWebviewPanel(
        'yoyoResponse',
        'Yoyo Response',
        vscode.ViewColumn.Beside,
        { enableScripts: true, retainContextWhenHidden: true }
    );

    responsePanel.onDidDispose(() => {
        responsePanel = undefined;
        panelInitialized = false;
        stopResponsePolling();
    });

    const responsePath = path.join(workspaceRoot, '.yoyo', 'response.md');

    // Ensure .yoyo directory exists
    const yoyoDir = path.join(workspaceRoot, '.yoyo');
    if (!fs.existsSync(yoyoDir)) {
        fs.mkdirSync(yoyoDir, { recursive: true });
    }

    // Initial content
    updateResponsePanel(responsePath);

    // Poll every 500ms — much more reliable than fs.watch on Windows
    startResponsePolling(responsePath);
}

function startResponsePolling(responsePath: string) {
    stopResponsePolling();
    responsePoller = setInterval(() => {
        try {
            if (!fs.existsSync(responsePath)) return;
            const stat = fs.statSync(responsePath);
            const mtime = stat.mtimeMs;
            if (mtime !== lastResponseMtime) {
                lastResponseMtime = mtime;
                const content = fs.readFileSync(responsePath, 'utf-8');
                // Only update if content actually changed (avoids flicker)
                if (content !== lastResponseContent) {
                    lastResponseContent = content;
                    updateResponsePanel(responsePath);
                }
            }
        } catch {
            // File may not exist yet, that's fine
        }
    }, 500);
}

function stopResponsePolling() {
    if (responsePoller) {
        clearInterval(responsePoller);
        responsePoller = undefined;
    }
}

let panelInitialized = false;

function updateResponsePanel(responsePath: string) {
    if (!responsePanel) return;

    let markdown = '';
    try {
        markdown = fs.readFileSync(responsePath, 'utf-8');
    } catch {
        markdown = '*Waiting for yoyo response...*';
    }

    if (!markdown.trim()) {
        markdown = '*Waiting for yoyo response...*';
    }

    if (!panelInitialized) {
        responsePanel.webview.html = getWebviewHtml();
        panelInitialized = true;
        // Small delay to let webview load before sending first message
        setTimeout(() => {
            responsePanel?.webview.postMessage({ type: 'update', markdown });
        }, 300);
    } else {
        responsePanel.webview.postMessage({ type: 'update', markdown });
    }
}

function getWebviewHtml(): string {
    return `<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Yoyo Response</title>
    <style>
        body {
            font-family: var(--vscode-font-family, -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif);
            font-size: var(--vscode-font-size, 13px);
            color: var(--vscode-foreground, #ccc);
            background: var(--vscode-editor-background, #1e1e1e);
            padding: 16px 24px;
            line-height: 1.6;
            max-width: 900px;
        }
        h1, h2, h3, h4 {
            color: var(--vscode-foreground, #eee);
            border-bottom: 1px solid var(--vscode-panel-border, #333);
            padding-bottom: 4px;
            margin-top: 24px;
        }
        h1 { font-size: 1.6em; }
        h2 { font-size: 1.3em; }
        h3 { font-size: 1.1em; }
        code {
            font-family: var(--vscode-editor-font-family, 'Consolas', 'Courier New', monospace);
            background: var(--vscode-textCodeBlock-background, #2d2d2d);
            padding: 2px 5px;
            border-radius: 3px;
            font-size: 0.9em;
        }
        pre {
            background: var(--vscode-textCodeBlock-background, #2d2d2d);
            border: 1px solid var(--vscode-panel-border, #444);
            border-radius: 6px;
            padding: 12px 16px;
            overflow-x: auto;
            position: relative;
        }
        pre code {
            background: none;
            padding: 0;
            font-size: 0.85em;
            line-height: 1.5;
        }
        pre[data-lang]::before {
            content: attr(data-lang);
            position: absolute;
            top: 4px;
            right: 8px;
            font-size: 0.7em;
            color: var(--vscode-descriptionForeground, #888);
            text-transform: uppercase;
        }
        table {
            border-collapse: collapse;
            width: 100%;
            margin: 12px 0;
        }
        th, td {
            border: 1px solid var(--vscode-panel-border, #444);
            padding: 8px 12px;
            text-align: left;
        }
        th {
            background: var(--vscode-textCodeBlock-background, #2d2d2d);
            font-weight: 600;
        }
        tr:nth-child(even) {
            background: rgba(255,255,255,0.03);
        }
        blockquote {
            border-left: 3px solid var(--vscode-textLink-foreground, #569cd6);
            margin: 12px 0;
            padding: 4px 16px;
            color: var(--vscode-descriptionForeground, #aaa);
        }
        a { color: var(--vscode-textLink-foreground, #569cd6); }
        ul, ol { padding-left: 24px; }
        li { margin: 4px 0; }
        hr {
            border: none;
            border-top: 1px solid var(--vscode-panel-border, #444);
            margin: 20px 0;
        }
        strong { color: var(--vscode-foreground, #eee); }
        .waiting {
            text-align: center;
            padding: 40px;
            color: var(--vscode-descriptionForeground, #888);
            font-style: italic;
        }
        ::-webkit-scrollbar { width: 8px; }
        ::-webkit-scrollbar-track { background: transparent; }
        ::-webkit-scrollbar-thumb {
            background: var(--vscode-scrollbarSlider-background, #555);
            border-radius: 4px;
        }
    </style>
</head>
<body>
    <div id="content"><p class="waiting">Waiting for yoyo response...</p></div>
    <script>
        const vscode = acquireVsCodeApi();
        const contentEl = document.getElementById('content');

        function esc(s) { return s.replace(/&/g,'&amp;').replace(/</g,'&lt;').replace(/>/g,'&gt;'); }

        function parseMarkdown(src) {
            const lines = src.split('\\n');
            let html = '';
            let i = 0;
            let inList = false;
            let listTag = '';

            function closeList() {
                if (inList) { html += '</' + listTag + '>'; inList = false; }
            }

            while (i < lines.length) {
                const line = lines[i];

                // Fenced code block
                const fenceMatch = line.match(/^\`\`\`(\\w*)/);
                if (fenceMatch) {
                    closeList();
                    const lang = fenceMatch[1] || '';
                    const codeLines = [];
                    i++;
                    while (i < lines.length && !lines[i].startsWith('\`\`\`')) {
                        codeLines.push(esc(lines[i]));
                        i++;
                    }
                    i++; // skip closing fence
                    const langAttr = lang ? ' data-lang="' + esc(lang) + '"' : '';
                    const langClass = lang ? ' class="language-' + esc(lang) + '"' : '';
                    html += '<pre' + langAttr + '><code' + langClass + '>' + codeLines.join('\\n') + '</code></pre>';
                    continue;
                }

                // Table (detect by | at start and end)
                if (line.trim().startsWith('|') && line.trim().endsWith('|')) {
                    closeList();
                    const tableRows = [];
                    while (i < lines.length && lines[i].trim().startsWith('|') && lines[i].trim().endsWith('|')) {
                        tableRows.push(lines[i]);
                        i++;
                    }
                    if (tableRows.length >= 2) {
                        html += '<table>';
                        // Header row
                        const headerCells = tableRows[0].split('|').filter(c => c.trim() !== '');
                        html += '<thead><tr>';
                        headerCells.forEach(c => { html += '<th>' + inlineMarkdown(c.trim()) + '</th>'; });
                        html += '</tr></thead>';
                        // Skip separator row (index 1)
                        html += '<tbody>';
                        for (let r = 2; r < tableRows.length; r++) {
                            const cells = tableRows[r].split('|').filter(c => c.trim() !== '');
                            html += '<tr>';
                            cells.forEach(c => { html += '<td>' + inlineMarkdown(c.trim()) + '</td>'; });
                            html += '</tr>';
                        }
                        html += '</tbody></table>';
                    }
                    continue;
                }

                // Headings
                const headingMatch = line.match(/^(#{1,6})\\s+(.*)/);
                if (headingMatch) {
                    closeList();
                    const level = headingMatch[1].length;
                    html += '<h' + level + '>' + inlineMarkdown(headingMatch[2]) + '</h' + level + '>';
                    i++;
                    continue;
                }

                // Horizontal rule
                if (/^(---|\*\*\*|___)\\s*$/.test(line.trim())) {
                    closeList();
                    html += '<hr>';
                    i++;
                    continue;
                }

                // Blockquote
                if (line.startsWith('> ')) {
                    closeList();
                    const quoteLines = [];
                    while (i < lines.length && lines[i].startsWith('> ')) {
                        quoteLines.push(lines[i].slice(2));
                        i++;
                    }
                    html += '<blockquote>' + quoteLines.map(l => '<p>' + inlineMarkdown(l) + '</p>').join('') + '</blockquote>';
                    continue;
                }

                // Unordered list
                const ulMatch = line.match(/^(\\s*)[-*+]\\s+(.*)/);
                if (ulMatch) {
                    if (!inList || listTag !== 'ul') {
                        closeList();
                        html += '<ul>';
                        inList = true;
                        listTag = 'ul';
                    }
                    html += '<li>' + inlineMarkdown(ulMatch[2]) + '</li>';
                    i++;
                    continue;
                }

                // Ordered list
                const olMatch = line.match(/^(\\s*)\\d+\\.\\s+(.*)/);
                if (olMatch) {
                    if (!inList || listTag !== 'ol') {
                        closeList();
                        html += '<ol>';
                        inList = true;
                        listTag = 'ol';
                    }
                    html += '<li>' + inlineMarkdown(olMatch[2]) + '</li>';
                    i++;
                    continue;
                }

                // Empty line
                if (line.trim() === '') {
                    closeList();
                    i++;
                    continue;
                }

                // Paragraph
                closeList();
                html += '<p>' + inlineMarkdown(line) + '</p>';
                i++;
            }
            closeList();
            return html;
        }

        function inlineMarkdown(text) {
            let s = esc(text);
            // Inline code
            s = s.replace(/\`([^\`]+)\`/g, '<code>$1</code>');
            // Bold + italic
            s = s.replace(/\\*\\*\\*(.+?)\\*\\*\\*/g, '<strong><em>$1</em></strong>');
            // Bold
            s = s.replace(/\\*\\*(.+?)\\*\\*/g, '<strong>$1</strong>');
            // Italic
            s = s.replace(/\\*(.+?)\\*/g, '<em>$1</em>');
            // Links
            s = s.replace(/\\[([^\\]]+)\\]\\(([^)]+)\\)/g, '<a href="$2">$1</a>');
            return s;
        }

        function renderMarkdown(raw) {
            if (!raw || !raw.trim()) {
                contentEl.innerHTML = '<p class="waiting">Waiting for yoyo response...</p>';
                return;
            }
            contentEl.innerHTML = parseMarkdown(raw);
            window.scrollTo(0, document.body.scrollHeight);
        }

        // Listen for updates from the extension
        window.addEventListener('message', event => {
            const msg = event.data;
            if (msg.type === 'update') {
                renderMarkdown(msg.markdown);
            }
        });
    </script>
</body>
</html>`;
}

// ─── Inline diff watching (existing) ────────────────────────────────

function startWatching(workspaceRoot: string) {
    const eventsFile = path.join(workspaceRoot, '.yoyo', 'tool_events.jsonl');

    // Ensure .yoyo directory exists
    const yoyoDir = path.join(workspaceRoot, '.yoyo');
    if (!fs.existsSync(yoyoDir)) {
        fs.mkdirSync(yoyoDir, { recursive: true });
    }

    // Initialize file size to current size (don't process old events)
    if (fs.existsSync(eventsFile)) {
        lastFileSize = fs.statSync(eventsFile).size;
    } else {
        lastFileSize = 0;
    }

    // Watch the .yoyo directory for changes to tool_events.jsonl
    try {
        watcher = fs.watch(yoyoDir, (eventType, filename) => {
            if (filename === 'tool_events.jsonl') {
                processNewEvents(eventsFile, workspaceRoot);
            }
        });
        watching = true;
        updateStatusBar();
        vscode.window.showInformationMessage('Yoyo: watching for file edits');
    } catch (err) {
        vscode.window.showErrorMessage(`Yoyo: failed to start watcher: ${err}`);
    }
}

function stopWatching() {
    if (watcher) {
        watcher.close();
        watcher = undefined;
    }
    watching = false;
    updateStatusBar();
    vscode.window.showInformationMessage('Yoyo: stopped watching');
}

function updateStatusBar() {
    if (watching) {
        statusBarItem.text = '$(eye) yoyo';
        statusBarItem.tooltip = 'Yoyo: watching for edits (click to stop)';
        statusBarItem.show();
    } else {
        statusBarItem.text = '$(eye-closed) yoyo';
        statusBarItem.tooltip = 'Yoyo: not watching (click to start)';
        statusBarItem.show();
    }
}

function processNewEvents(eventsFile: string, workspaceRoot: string) {
    if (!fs.existsSync(eventsFile)) return;

    const stat = fs.statSync(eventsFile);
    if (stat.size <= lastFileSize) return;

    // Read only the new bytes
    const fd = fs.openSync(eventsFile, 'r');
    const newBytes = Buffer.alloc(stat.size - lastFileSize);
    fs.readSync(fd, newBytes, 0, newBytes.length, lastFileSize);
    fs.closeSync(fd);
    lastFileSize = stat.size;

    const newContent = newBytes.toString('utf-8');
    const lines = newContent.split('\n').filter(l => l.trim().length > 0);

    for (const line of lines) {
        try {
            const event: ToolEvent = JSON.parse(line);
            if (event.tool === 'edit_file' && event.path && event.old_text !== undefined) {
                showInlineDiff(event, workspaceRoot);
            }
        } catch {
            // Skip malformed lines
        }
    }
}

async function showInlineDiff(event: ToolEvent, workspaceRoot: string) {
    const filePath = path.isAbsolute(event.path)
        ? event.path
        : path.join(workspaceRoot, event.path);

    // Create a virtual document with the old content for diff comparison
    const oldUri = vscode.Uri.parse(
        `yoyo-before:${event.path}?ts=${encodeURIComponent(event.ts)}`
    );
    const newUri = vscode.Uri.file(filePath);

    // Register a content provider for the "before" state
    const provider = new YoyoDiffContentProvider(event);
    const registration = vscode.workspace.registerTextDocumentContentProvider('yoyo-before', provider);

    const fileName = path.basename(event.path);
    const title = `yoyo edit: ${fileName}`;

    // Show the diff
    await vscode.commands.executeCommand('vscode.diff', oldUri, newUri, title, {
        preview: true,
        viewColumn: vscode.ViewColumn.Active,
    });

    // Clean up the provider after a delay
    setTimeout(() => registration.dispose(), 60000);
}

async function showLastDiff(workspaceRoot: string) {
    const eventsFile = path.join(workspaceRoot, '.yoyo', 'tool_events.jsonl');
    if (!fs.existsSync(eventsFile)) {
        vscode.window.showInformationMessage('Yoyo: no tool events found (.yoyo/tool_events.jsonl)');
        return;
    }

    const content = fs.readFileSync(eventsFile, 'utf-8');
    const lines = content.split('\n').filter(l => l.trim().length > 0);

    // Find the last edit_file event
    for (let i = lines.length - 1; i >= 0; i--) {
        try {
            const event: ToolEvent = JSON.parse(lines[i]);
            if (event.tool === 'edit_file') {
                await showInlineDiff(event, workspaceRoot);
                return;
            }
        } catch {
            continue;
        }
    }

    vscode.window.showInformationMessage('Yoyo: no edit events found');
}

/**
 * Content provider that reconstructs the file state *before* an edit.
 * Reads the current file and reverses the edit (replaces new_text with old_text)
 * to show what the file looked like before yoyo changed it.
 */
class YoyoDiffContentProvider implements vscode.TextDocumentContentProvider {
    private event: ToolEvent;

    constructor(event: ToolEvent) {
        this.event = event;
    }

    provideTextDocumentContent(_uri: vscode.Uri): string {
        const filePath = path.isAbsolute(this.event.path)
            ? this.event.path
            : path.join(
                vscode.workspace.workspaceFolders?.[0]?.uri.fsPath ?? '',
                this.event.path
            );

        try {
            // Read the current file content
            let content = fs.readFileSync(filePath, 'utf-8');

            // Reverse the edit: replace new_text back with old_text
            // This reconstructs the "before" state
            if (this.event.new_text && content.includes(this.event.new_text)) {
                content = content.replace(this.event.new_text, this.event.old_text);
            } else {
                // If new_text isn't found (file changed again), just show old_text
                // as a standalone document
                return this.event.old_text || '(empty)';
            }

            return content;
        } catch {
            return this.event.old_text || '(file not found)';
        }
    }
}

export function deactivate() {
    stopWatching();
    stopResponsePolling();
    if (responsePanel) {
        responsePanel.dispose();
        responsePanel = undefined;
    }
}
