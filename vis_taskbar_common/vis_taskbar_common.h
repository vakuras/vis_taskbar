#pragma once

#include <Windows.h>
#include <stdio.h>
#include <tchar.h>
#include <gl\GL.h>
#include <gl\GLU.h>
#include <cmath>

#define Trace(format, ...) vis_taskbar_common::TraceImpl(__FUNCTION__, __LINE__, 0, format, __VA_ARGS__)
#define TraceErrWithCode(errCode, format, ...) vis_taskbar_common::TraceImpl(__FUNCTION__, __LINE__, errCode, format, __VA_ARGS__)
#define TraceErr(format, ...) vis_taskbar_common::TraceImpl(__FUNCTION__, __LINE__, GetLastError(), format, __VA_ARGS__)

typedef struct{
	float									R;
	float									G;
	float									B;
} VISRGB;

typedef struct{
	bool									FullTaskbar;
	VISRGB									RGBTop;
	VISRGB									RGBBottom;
	VISRGB									RGBPeaks;
	DWORD									StepMultiplier;
	DWORD									SleepTime;
	bool									Bars;
} SETTINGS, *PSETTINGS;

class vis_taskbar_common
{
public:
	vis_taskbar_common();
	~vis_taskbar_common();

private:
	SETTINGS								m_Settings;
	HWND									m_hWnd;
	HDC										m_hDC;
	HGLRC									m_hGLRC;
	HWND									m_hTaskBar;
	PSHORT									m_VisFalloff;
	PSHORT									m_VisPeakFalloff;
	PUCHAR									m_NewValues;
	HANDLE									m_hThread;
	PHANDLE									m_pNotifyEvent;
	RECT									m_RectInner;
	RECT									m_RectOuter;
	HWND									m_hTaskList;
	DWORD									m_DataSize;
	bool									m_Started;

	bool									InitGL();
	void									Clean();
	bool									InitWindow();
	bool									LocateTaskBar();
	bool									GetWindowRects(RECT & outer, RECT & inner);
	bool									Render();
	bool									HandleWindowRect();
	DWORD									RenderThread();

	static BOOL CALLBACK					FindChildProc(HWND hWndChild, LPARAM lParam);
	static DWORD							RenderThreadWrapper(PVOID lpThreadParameter);
	static LRESULT CALLBACK					WindowProcedure(HWND hWnd, UINT Msg,  WPARAM wParam, LPARAM lParam);
	
	static void								GetDllNameChangeExt(PTCHAR path, const PTCHAR ext, DWORD dwSize);
	static bool								LogToFile;
	static FILE*							LogFile;
	static PTCHAR							AppName;

protected:
	virtual bool							FillSpectrumData(PUCHAR values) = 0;
	bool									LoadConfiguration();
	bool									SaveConfiguration();
	SETTINGS								GetSettings();
	void									SetSettings(SETTINGS settings);

	void									SetTaskList(HWND hTaskList);
	void									ResizeWindow();

	virtual bool							StartImpl(DWORD dataSize, PHANDLE pNotifyEvent);
	virtual void							StopImpl();

	bool									IsMeetRequirement();
	PUCHAR									GetValuesBuffer();
	bool									IsStarted();

public:
	static void								InitTrace();
	static void								DeInitTrace();
	static void								SetAppName(PTCHAR appName);
	static void								TraceImpl(const PCHAR functionName, int lineNumber, DWORD errCode, const PTCHAR format,...);
};