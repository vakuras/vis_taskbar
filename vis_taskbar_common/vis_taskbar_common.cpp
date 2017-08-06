#include "vis_taskbar_common.h"
#include <Dwmapi.h>

#pragma comment(lib, "opengl32.lib") 
#pragma comment(lib, "glu32.lib") 
#pragma comment(lib, "Dwmapi.lib") 

#define CLASS_NAME_TASKBAR					L"Shell_TrayWnd"
#define CAPTION_TASKLIST					L"Running applications"
#define VIS_CLASS							L"VIS_CLASS"
#define VIS_TITLE							L"VIS_TASKBAR"

EXTERN_C IMAGE_DOS_HEADER __ImageBase;

bool vis_taskbar_common::LogToFile;
FILE* vis_taskbar_common::LogFile = NULL;
PTCHAR vis_taskbar_common::AppName = L"";

//members

vis_taskbar_common::vis_taskbar_common()
{
	m_VisPeakFalloff = NULL;
	m_VisFalloff = NULL;
	m_NewValues = NULL;
	m_hThread = NULL;
	m_hWnd = NULL;
	m_hDC = NULL;
	m_hTaskBar = NULL;
	m_hTaskList = NULL;

	m_DataSize = 0;

	ZeroMemory(&m_Settings, sizeof(m_Settings));
	ZeroMemory(&m_RectInner, sizeof(m_RectInner));
	ZeroMemory(&m_RectOuter, sizeof(m_RectOuter));

	m_Started = false;
}

vis_taskbar_common::~vis_taskbar_common()
{
	StopImpl();

	if (m_NewValues)
	{
		delete [] m_NewValues;
		m_NewValues = NULL;
	}

	if (m_VisPeakFalloff)
	{
		delete [] m_VisPeakFalloff;
		m_VisPeakFalloff = NULL;
	}

	if (m_VisFalloff)
	{
		delete [] m_VisFalloff;
		m_VisFalloff = NULL;
	}

}

SETTINGS vis_taskbar_common::GetSettings()
{
	Trace(L"");
	return m_Settings;
}

void vis_taskbar_common::SetSettings(SETTINGS settings)
{
	Trace(L"");
	m_Settings = settings;
}

void vis_taskbar_common::SetTaskList(HWND hTaskList)
{
	Trace(L"");
	m_hTaskList = hTaskList;
}

bool vis_taskbar_common::SaveConfiguration()
{
	FILE * file = NULL;

	TCHAR path[MAX_PATH];
	GetDllNameChangeExt(path, L"cfg", MAX_PATH);

	__try
	{
		if (_tfopen_s(&file, path, TEXT("wb")))
		{
			TraceErr(L"_tfopen_s failed");
			return false;
		}
		if (fwrite(&m_Settings, sizeof(SETTINGS), 1, file) != 1)
		{
			Trace(L"fwrite failed");
			return false;
		}

		return true;
	}
	__finally
	{
		if (file)
			fclose(file);
	}
}

bool vis_taskbar_common::LoadConfiguration()
{
	FILE * file = NULL;

	TCHAR path[MAX_PATH];
	GetDllNameChangeExt(path, L"cfg", MAX_PATH);

	__try
	{
		m_Settings.SleepTime = 15;
		m_Settings.FullTaskbar = TRUE;
		m_Settings.RGBTop.R = 1.0f;
		m_Settings.RGBTop.G = 1.0f;
		m_Settings.RGBTop.B = 0;
		m_Settings.RGBBottom.R = 1.0f;
		m_Settings.RGBBottom.G = 0;
		m_Settings.RGBBottom.B = 0;
		m_Settings.RGBPeaks.R = 1.0f;
		m_Settings.RGBPeaks.G = 1.0f;
		m_Settings.RGBPeaks.B = 1.0f;
		m_Settings.StepMultiplier = 1;

		if (_tfopen_s(&file, path, TEXT("rb")))
		{
			TraceErr(L"_tfopen_s failed");
			return false;
		}

		SETTINGS tmp;
		if (fread_s(&tmp, sizeof(SETTINGS), sizeof(SETTINGS), 1, file) != 1)
		{
			Trace(L"fread_s failed");
			return false;
		}

		m_Settings = tmp;

		return true;
	}
	__finally
	{
		if (file)
			fclose(file);
	}
}

DWORD vis_taskbar_common::RenderThreadWrapper(PVOID lpThreadParameter)
{
	return ((vis_taskbar_common*)lpThreadParameter)->RenderThread();
}

bool vis_taskbar_common::StartImpl(DWORD dataSize, PHANDLE pNotifyEvent)
{
	__try
	{
		Trace(L"Begin");

		if (m_Started)
			return false;

		m_hWnd = NULL;
		m_hDC = NULL;
		m_hGLRC = NULL;
		m_hTaskBar = NULL;
		m_hTaskList = NULL;

		if (!LoadConfiguration())
			Trace(L"Unable to load configuration, using default values");
		else
			Trace(L"Configuration loaded successfully");

		if (!LocateTaskBar())
		{
			Trace(L"Unable to locate taskbar!");
			return false;
		}

		if (m_DataSize != dataSize)
		{
			m_DataSize = dataSize;

			if (m_NewValues)
				delete [] m_NewValues;

			if (m_VisFalloff)
				delete [] m_VisFalloff;

			if (m_VisPeakFalloff)
				delete [] m_VisPeakFalloff;

			m_NewValues = new UCHAR[dataSize];
			m_VisFalloff = new SHORT[dataSize];
			m_VisPeakFalloff = new SHORT[dataSize];
		}

		ZeroMemory(m_NewValues, dataSize);
		ZeroMemory(m_VisFalloff, dataSize);
		ZeroMemory(m_VisPeakFalloff, dataSize);

		m_pNotifyEvent = pNotifyEvent;

		m_Started = true;
		m_hThread = CreateThread(NULL, 100, (LPTHREAD_START_ROUTINE)RenderThreadWrapper, this, 0, NULL);

		if (!m_hThread)
		{
			TraceErr(L"Unable to create rendering thread");
			m_Started = false;
			return false;
		}

		return true;
	}
	__finally
	{
		Trace(L"End");
	}
}

void vis_taskbar_common::StopImpl()
{	
	Trace(L"Begin");

	m_Started = false;

	if (m_hThread)
	{
		if (WaitForSingleObject(m_hThread, 5000) != WAIT_OBJECT_0)
		{
			Trace(L"Thread wait timeout occured, trying to terminate");
			if (!TerminateThread(m_hThread, EXIT_FAILURE))
				TraceErr(L"Unable to terminate thread");
		}

		CloseHandle(m_hThread);
		m_hThread = NULL;
	}

	m_pNotifyEvent = NULL;

	Trace(L"End");
}

/* graphics */

bool vis_taskbar_common::HandleWindowRect()
{
	RECT rectInner;
	RECT rectOuter;
	if (GetWindowRects(rectOuter, rectInner))
	{
		if (memcmp(&rectInner, &m_RectInner, sizeof(RECT)) != 0 || memcmp(&rectOuter, &m_RectOuter, sizeof(RECT)) != 0)
		{
			Trace(L"New taskbar location retrieved");

			if (!SetWindowPos(m_hWnd, HWND_TOP, rectOuter.left, rectOuter.top, rectOuter.right - rectOuter.left, rectOuter.bottom - rectOuter.top, 0))
			{
				TraceErr(L"SetWindowPos failed");
				return false;
			}

			memcpy(&m_RectInner, &rectInner, sizeof(RECT));
			memcpy(&m_RectOuter, &rectOuter, sizeof(RECT));
		}
	}
	else
	{
		return false;
	}

	return true;
}

bool vis_taskbar_common::Render()
{
	int height;
	int width;
	int step;
	int center;
	int nValue1st;
	int nValue2nd;
	bool newValue;
	UCHAR value;
	DWORD targetIndex;
	DWORD halfDataSize = m_DataSize / 2;
	DWORD halfDataSizeMinusOne = halfDataSize - 1;

	if (!HandleWindowRect())
	{
		Trace(L"HandleWindowRect failed");
		return false;
	}

	newValue = FillSpectrumData(m_NewValues);

	for(DWORD i=0; i<m_DataSize; i++)
	{		
		value = m_NewValues[i];

		if (newValue && m_VisFalloff[i] < value)
		{
			m_VisFalloff[i] = value;
		}
		else
		{
			m_VisFalloff[i]-= 7;

			if (m_VisFalloff[i] < 0)
				m_VisFalloff[i] = 0;
		}

		if (newValue && m_VisPeakFalloff[i] < value)
		{
			m_VisPeakFalloff[i] = value;
		}
		else
		{
			m_VisPeakFalloff[i]-=2;

			if (m_VisPeakFalloff[i] < 0)
				m_VisPeakFalloff[i] = 0;
		}
	}

	if (newValue && m_pNotifyEvent)
		SetEvent(*m_pNotifyEvent);

	height = m_RectInner.bottom - m_RectInner.top;
	width = m_RectInner.right - m_RectInner.left;

	glMatrixMode(GL_PROJECTION);
	glLoadIdentity();
	glOrtho(0, m_RectOuter.right - m_RectOuter.left, m_RectOuter.bottom - m_RectOuter.top, 0, 0, 1);
	glDisable(GL_DEPTH_TEST);
	glMatrixMode (GL_MODELVIEW);
	glLoadIdentity();

	glTranslatef(0.375f, 0.375f, 0.0f);

	glClearColor(0.0f, 0.0f, 0.0f, 0.0f);
	glClear(GL_COLOR_BUFFER_BIT);

	step = (int) (ceil((double)(m_RectInner.right - m_RectInner.left) / m_DataSize) * m_Settings.StepMultiplier);
	center = (m_RectInner.left - m_RectOuter.left) + ((m_RectInner.right - m_RectInner.left) / 2);

	targetIndex =  ((center - (m_RectInner.left - m_RectOuter.left)) / step) - 1;
	targetIndex = halfDataSizeMinusOne > targetIndex ? targetIndex : halfDataSizeMinusOne;

	if (m_Settings.Bars)
		targetIndex++;

	//left spectrum
	glBegin(GL_QUADS);
	for(DWORD i=0; i<targetIndex; i++)
	{
		nValue1st = (m_VisFalloff[i] * height / 255);

		if (m_Settings.Bars)
			nValue2nd = nValue1st;
		else
			nValue2nd = (m_VisFalloff[i + 1] * height / 255);

		//top left
		glColor3f(m_Settings.RGBTop.R, m_Settings.RGBTop.G, m_Settings.RGBTop.B);
		glVertex2i(center - (i + 1) * step, height - nValue2nd);
		//top right
		glVertex2i(center - i * step, height - nValue1st); 

		//bottom right
		glColor3f(m_Settings.RGBBottom.R, m_Settings.RGBBottom.G, m_Settings.RGBBottom.B);
		glVertex2i(center - i * step, height);
		//bottom left
		glVertex2i(center - (i + 1) * step, height);
	}
	glEnd();
	
	glColor3f(m_Settings.RGBPeaks.R, m_Settings.RGBPeaks.G, m_Settings.RGBPeaks.B);
	glBegin(GL_LINE_STRIP);
	for(DWORD i=0; i<targetIndex; i++)
	{
		nValue1st = (m_VisPeakFalloff[i] * height / 255);
		glVertex2i(center - i * step, height - nValue1st); 
	}
	glEnd();

	targetIndex = ((m_RectInner.right - center) / step) + 1;
	targetIndex = halfDataSizeMinusOne > targetIndex ? targetIndex : halfDataSizeMinusOne;

	if (m_Settings.Bars)
		targetIndex++;

	//right spectrum
	glBegin(GL_QUADS);
	for(DWORD i=0; i<targetIndex; i++)
	{
		nValue1st = (m_VisFalloff[i + halfDataSize] * height / 255);

		if (m_Settings.Bars)
			nValue2nd = nValue1st;
		else
			nValue2nd = (m_VisFalloff[i + 1 + halfDataSize] * height / 255);

		//top left
		glColor3f(m_Settings.RGBTop.R, m_Settings.RGBTop.G, m_Settings.RGBTop.B);
		glVertex2i(center + i * step, height - nValue1st);
		//top right
		glVertex2i(center + (i + 1) * step, height - nValue2nd); 

		//bottom right
		glColor3f(m_Settings.RGBBottom.R, m_Settings.RGBBottom.G, m_Settings.RGBBottom.B);
		glVertex2i(center + (i + 1) * step, height);
		//bottom left
		glVertex2i(center + i * step, height);
	}
	glEnd();

	
	glColor3f(m_Settings.RGBPeaks.R, m_Settings.RGBPeaks.G, m_Settings.RGBPeaks.B);
	glBegin(GL_LINE_STRIP);
	for(DWORD i=0; i<targetIndex; i++)
	{
		nValue1st = (m_VisPeakFalloff[i + halfDataSize] * height / 255);
		glVertex2i(center + i * step, height - nValue1st); 
	}
	glEnd();

	if (!SwapBuffers(m_hDC))
	{
		TraceErr(L"SwapBuffers failed");
		return false;
	}

	return true;
}

bool vis_taskbar_common::GetWindowRects(RECT & outer, RECT & inner)
{
	bool rectLoaded = false;
	do
	{
		if (!GetWindowRect(m_hTaskBar, &outer))
		{
			DWORD err = GetLastError();

			if (err == ERROR_INVALID_WINDOW_HANDLE)
			{
				if (!LocateTaskBar())
					return false;

				continue;
			}

			TraceErrWithCode(err ,L"GetWindowRect [1st] failed");
			return false;
		}

		if (m_Settings.FullTaskbar)
			inner = outer;
		else
		{
			if (!GetWindowRect(m_hTaskList, &inner))
			{
				DWORD err = GetLastError();

				if (err == ERROR_INVALID_WINDOW_HANDLE)
				{
					if (!LocateTaskBar())
						return false;

					continue;
				}

				TraceErrWithCode(err, L"GetWindowRect [2st] failed");
				return false;
			}
		}

		rectLoaded = true;
	} while(!rectLoaded);

	return true;
}


bool vis_taskbar_common::LocateTaskBar()
{
	__try
	{
		Trace(L"Begin");

		m_hTaskBar = FindWindow(CLASS_NAME_TASKBAR, NULL);

		if (!m_hTaskBar)
		{
			TraceErr(L"FindWindow failed");
			return false;
		}

		EnumChildWindows(m_hTaskBar, FindChildProc, (LPARAM)this);

		if (!m_hTaskList)
		{
			TraceErr(L"EnumChildWindows failed");
			return false;
		}

		return true;
	}
	__finally
	{
		Trace(L"End");
	}
}

DWORD vis_taskbar_common::RenderThread()
{
	__try
	{
		Trace(L"Begin");

		if (!InitWindow())
			return EXIT_FAILURE;

		if (!InitGL())
			return EXIT_FAILURE;

		MSG msg;
		while(m_Started)
		{
			while (PeekMessage(&msg, NULL, 0, 0, PM_REMOVE) == TRUE)
			{
				TranslateMessage(&msg);
				DispatchMessage(&msg);			
			}

			if (!Render())
			{
				Trace(L"Render failed, exiting");
				return EXIT_FAILURE;
			}

			Sleep(m_Settings.SleepTime); //sleep a bit
		}

		return EXIT_SUCCESS;
	}
	__finally
	{
		Clean();
		Trace(L"End");
	}
}


void vis_taskbar_common::ResizeWindow()
{
	Trace(L"Begin");

	RECT rect;
	GetWindowRect(m_hWnd, &rect);

	glViewport(0, 0, rect.right - rect.left, rect.bottom - rect.top);
	glMatrixMode(GL_PROJECTION);
	glLoadIdentity();

	gluPerspective(45.0f,(GLfloat)(rect.right - rect.left)/(GLfloat)(rect.bottom - rect.top),0.1f,100.0f);
	glMatrixMode(GL_MODELVIEW);
	glLoadIdentity();

	Trace(L"End");
}

LRESULT CALLBACK vis_taskbar_common::WindowProcedure(HWND hWnd, UINT Msg,  WPARAM wParam, LPARAM lParam)
{
    switch(Msg)
    {
	case WM_CREATE:
		SetWindowLongPtr(hWnd, GWLP_USERDATA, (LONG)((LPCREATESTRUCT)lParam)->lpCreateParams);
		break;

	case WM_SIZE:
		((vis_taskbar_common*)GetWindowLongPtr(hWnd, GWLP_USERDATA))->ResizeWindow();
		break;

    case WM_DESTROY:
        PostQuitMessage(WM_QUIT);
        break;

    default:
        return DefWindowProc(hWnd, Msg, wParam, lParam);
    }
    return 0;
}

BOOL CALLBACK vis_taskbar_common::FindChildProc(HWND hWndChild, LPARAM lParam)
{
	TCHAR tWindowText[255];
	vis_taskbar_common* self = (vis_taskbar_common*)lParam;
	GetWindowText(hWndChild, tWindowText, 255);

	if (!_tcscmp(tWindowText, CAPTION_TASKLIST))
	{
		self->SetTaskList(hWndChild);
		return FALSE;
	}

	return TRUE;	
}

bool vis_taskbar_common::InitWindow()
{
	__try
	{
		Trace(L"Begin");

		RECT rect;
		WNDCLASSEX WndClsEx;

		if (!GetWindowRect(m_hTaskBar, &rect))
		{
			TraceErr(L"GetWindowRect failed");
			return false;
		}

		WndClsEx.cbSize        = sizeof(WNDCLASSEX);
		WndClsEx.style         = CS_HREDRAW | CS_VREDRAW;
		WndClsEx.lpfnWndProc   = WindowProcedure;
		WndClsEx.cbClsExtra    = 0;
		WndClsEx.cbWndExtra    = 0;
		WndClsEx.hIcon         = LoadIcon(NULL, IDI_APPLICATION);
		WndClsEx.hCursor       = LoadCursor(NULL, IDC_ARROW);
		WndClsEx.hbrBackground = (HBRUSH)GetStockObject(BLACK_BRUSH);
		WndClsEx.lpszMenuName  = NULL;
		WndClsEx.lpszClassName = VIS_CLASS;
		WndClsEx.hInstance     = GetModuleHandle(NULL);
		WndClsEx.hIconSm       = LoadIcon(NULL, IDI_APPLICATION);

		if (!RegisterClassEx(&WndClsEx))
		{
			TraceErr(L"RegisterClassEx failed");
			return false;
		}

		m_hWnd = CreateWindowEx(
			WS_EX_TOOLWINDOW,
			VIS_CLASS,
			VIS_TITLE,
			WS_POPUP,
			rect.left,
			rect.top,
			rect.right - rect.left,
			rect.bottom - rect.top,
			NULL,
			NULL,
			GetModuleHandle(NULL),
			this);

		if (!m_hWnd)
		{
			TraceErr(L"CreateWindow failed");
			return false;
		}

		ShowWindow(m_hWnd, SW_SHOWNORMAL);
		UpdateWindow(m_hWnd);

		return true;
	}
	__finally
	{
		Trace(L"End");
	}
}

void vis_taskbar_common::Clean()
{
	Trace(L"Begin");

	if (!wglMakeCurrent(NULL,NULL))
		TraceErr(L"wglMakeCurrent failed");

	if (m_hGLRC)
		if (!wglDeleteContext(m_hGLRC))
			TraceErr(L"wglDeleteContext failed");

	if (m_hDC)
		if (!ReleaseDC(m_hWnd,m_hDC))
			Trace(L"ReleaseDC failed");

	if (m_hWnd)
		if (!DestroyWindow(m_hWnd))
			TraceErr(L"UnregisterClass failed");

	if (!UnregisterClass(VIS_CLASS,GetModuleHandle(NULL)))
		TraceErr(L"UnregisterClass failed");

	m_hWnd = NULL;
	m_hGLRC = NULL;
	m_hDC = NULL;

	Trace(L"End");
}

bool vis_taskbar_common::InitGL()
{
	__try
	{
		Trace(L"Begin");

		GLuint pixelFormat;

		PIXELFORMATDESCRIPTOR pfd=
		{
			sizeof(PIXELFORMATDESCRIPTOR),
			1,
			PFD_DRAW_TO_WINDOW | PFD_SUPPORT_OPENGL | PFD_DOUBLEBUFFER,
			PFD_TYPE_RGBA,
			32,
			0, 0, 0, 0, 0, 0,
			0,
			0,
			0,
			0, 0, 0, 0,
			16,
			0,
			0,
			PFD_MAIN_PLANE,
			0,
			0, 0, 0
		};

		if (!(m_hDC=GetDC(m_hWnd)))
		{
			TraceErr(L"Can't create a gL device context");
			return false;
		}

		if (!(pixelFormat=ChoosePixelFormat(m_hDC,&pfd)))
		{
			TraceErr(L"Can't find a suitable pixelformat");
			return false;
		}

		if(!SetPixelFormat(m_hDC,pixelFormat,&pfd))
		{
			TraceErr(L"Can't set the pixelformat");
			return false;
		}

		if (!(m_hGLRC=wglCreateContext(m_hDC)))
		{
			TraceErr(L"Can't create a gl rendering context");
			return false;
		}

		if(!wglMakeCurrent(m_hDC,m_hGLRC))
		{
			TraceErr(L"Can't activate the gl rendering context");
			return false;
		}

		glClearColor(0, 0, 0, 0);
		glClearDepth(1.0f);
		glHint(GL_LINE_SMOOTH_HINT,GL_NICEST);
		glEnable(GL_BLEND);
		glBlendFunc(GL_SRC_ALPHA, GL_ONE_MINUS_SRC_ALPHA);

		return true;
	}
	__finally
	{
		Trace(L"End");
	}
}

PUCHAR vis_taskbar_common::GetValuesBuffer()
{
	return m_NewValues;
}

bool vis_taskbar_common::IsStarted()
{
	return m_Started;
}

//static

void vis_taskbar_common::SetAppName(PTCHAR appName)
{
	vis_taskbar_common::AppName = appName;
}

void vis_taskbar_common::GetDllNameChangeExt(PTCHAR path, const PTCHAR ext, DWORD dwSize)
{
	GetModuleFileName((HINSTANCE)&__ImageBase, path, dwSize);
	PTCHAR pChr = _tcsrchr(path, '.');
	
	if (pChr)
		*(pChr+1) = NULL;

	_tcscat_s(path, dwSize, ext);
}

void vis_taskbar_common::InitTrace()
{
	TCHAR path[MAX_PATH];
	GetDllNameChangeExt(path, L"log", MAX_PATH);

	if (_tfopen_s(&LogFile, path, TEXT("w, ccs=UNICODE")))
	{
		LogToFile = false;
		return;
	}

	LogToFile = true;
}

void vis_taskbar_common::DeInitTrace()
{
	LogToFile = false;

	if (LogFile)
		fclose(LogFile);

	LogFile = NULL;
}

void vis_taskbar_common::TraceImpl(const PCHAR functionName, int lineNumber, DWORD errCode, const PTCHAR format,...)
{
	#define TRACEMAXSTRING 1024
	static TCHAR szBuffer1[TRACEMAXSTRING];
	static TCHAR szBuffer2[TRACEMAXSTRING];

    va_list args;
    va_start(args,format);
    _vsntprintf_s(szBuffer1, TRACEMAXSTRING, format, args);
    va_end(args);

	if (errCode)
		_stprintf_s(szBuffer2, TRACEMAXSTRING, _T("%s | %S(%d) | %s | Error[%d]\n"), AppName, functionName, lineNumber, szBuffer1, errCode);
	else
		_stprintf_s(szBuffer2, TRACEMAXSTRING, _T("%s | %S(%d) | %s\n"), AppName, functionName, lineNumber, szBuffer1);

    OutputDebugString(szBuffer2);

	if (!LogToFile)
		return;

	if (_ftprintf(LogFile, _T("%s"), szBuffer2) > -1)
		return;
		
	LogToFile = false;
	DeInitTrace();
}

bool vis_taskbar_common::IsMeetRequirement()
{
	OSVERSIONINFO osvi;

    ZeroMemory(&osvi, sizeof(OSVERSIONINFO));
    osvi.dwOSVersionInfoSize = sizeof(OSVERSIONINFO);

    GetVersionEx(&osvi);

	if (osvi.dwMajorVersion != 6 && osvi.dwMinorVersion != 1)
	{
		Trace(L"OS is not Windows 7");
		return false;
	}

	BOOL dwmEnabled = FALSE;
	HRESULT hr = DwmIsCompositionEnabled(&dwmEnabled);

	if (FAILED(hr))
	{
		Trace(L"DwmIsCompositionEnabled failed, hr:[%x]", hr);
		return false;
	}

	if (!dwmEnabled)
	{
		Trace(L"Aero is not enabled");
		return false;
	}

	return true;
}