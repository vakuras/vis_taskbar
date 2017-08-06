#include "win7_vis_taskbar.h"
#include "resource.h"
#include <CommCtrl.h>

EXTERN_C IMAGE_DOS_HEADER __ImageBase;

// members

win7_vis_taskbar::win7_vis_taskbar()
{
	ZeroMemory(&m_SettingsClone, sizeof(m_SettingsClone));
	ZeroMemory(&m_NotifyIcon, sizeof(m_NotifyIcon));

	m_BufferPosition = 0;
	m_BufferSize = 0;
	m_DataIsAvailable = false;

	m_pMMDevice = NULL;
	m_pAudioClient = NULL;
	m_pWaveFormatEx = NULL;
	m_pAudioCaptureClient = NULL;
	m_pBuffer = NULL;

	m_pFFTSpectrum = NULL;
	m_hAudioEvent = NULL;
	m_hAudioCaptureThread = NULL;
}

win7_vis_taskbar::~win7_vis_taskbar()
{
	Uninitialize();

	if (!Shell_NotifyIcon(NIM_DELETE, &m_NotifyIcon))
		Trace(L"Shell_NotifyIcon(NIM_DELETE) failed, hr:[0x%x]", HRESULT_FROM_WIN32(GetLastError()));

	if (m_NotifyIcon.hIcon)
		DestroyIcon(m_NotifyIcon.hIcon);

	if (m_hPreferencesDialog)
	{
		DestroyWindow(m_hPreferencesDialog);
		m_hPreferencesDialog = NULL;
	}
}

bool win7_vis_taskbar::BuildUI(HINSTANCE hInstance)
{
	if (!IsMeetRequirement())
	{
		Trace(L"OS doesn't meet requirements");
		MessageBox(NULL, L"You OS doesn't meet the requirements needed for the plugin to work!", L"win7_vis_taskbar", MB_OK | MB_ICONERROR);
		return false;
	}

	m_hPreferencesDialog = CreateDialogParam(
		hInstance,
        MAKEINTRESOURCE(IDD_TBPREFS),
        NULL,
        (DLGPROC)PreferencesPageWindowProcedure,
		(LPARAM)this);

	if (!m_hPreferencesDialog)
	{
		Trace(L"CreateDialogParam failed, hr:[0x%x]", HRESULT_FROM_WIN32(GetLastError()));
		return false;
	}

	m_NotifyIcon.cbSize = sizeof(m_NotifyIcon);
	m_NotifyIcon.uFlags = NIF_ICON | NIF_TIP | NIF_MESSAGE;
	m_NotifyIcon.hIcon = LoadIcon(hInstance, MAKEINTRESOURCE(IDI_ICON));

	if (!m_NotifyIcon.hIcon)
	{
		Trace(L"LoadIcon failed, hr:[0x%x]", HRESULT_FROM_WIN32(GetLastError()));
		return false;
	}

	m_NotifyIcon.uID = ICON_UID;
	m_NotifyIcon.hWnd = m_hPreferencesDialog;
	m_NotifyIcon.uCallbackMessage = ICON_MSG; 
	_tcscpy_s(m_NotifyIcon.szTip, APPNAME);

	if (!Shell_NotifyIcon(NIM_ADD, &m_NotifyIcon))
	{
		Trace(L"Shell_NotifyIcon(NIM_ADD) failed, hr:[0x%x]", HRESULT_FROM_WIN32(GetLastError()));
		return false;
	}

	return true;
}

bool win7_vis_taskbar::Initialize()
{
	m_BufferPosition = 0;
	m_DataIsAvailable = false;

	m_hAudioEvent = CreateEvent(NULL, TRUE, TRUE, NULL);
	if (!m_hAudioEvent)
	{
		Trace(L"CreateEvent failed, hr:[0x%x]", HRESULT_FROM_WIN32(GetLastError()));
		return false;
	}

	m_pFFTSpectrum = fft_spectrum::Create(FFT_SIZE);

	if (!StartImpl(m_pFFTSpectrum->GetBinSize() * 2, &m_hAudioEvent))
		return false;

	if (!InitializeAudio())
		return false;

	return true;
}

void win7_vis_taskbar::Uninitialize()
{
	StopImpl();
	UninitializeAudio();

	if (m_hAudioEvent)
	{
		CloseHandle(m_hAudioEvent);
		m_hAudioEvent = NULL;
	}

	if (m_pFFTSpectrum)
	{
		delete m_pFFTSpectrum;
		m_pFFTSpectrum = NULL;
	}
}

bool win7_vis_taskbar::StutteringFix(IMMDevice* pMMDevice)
{
	IAudioClient* pAudioClient = NULL;
	PWAVEFORMATEX pWaveFormatEx = NULL;
	IAudioRenderClient* pAudioRenderClient = NULL;

	__try
	{
		HRESULT hr = pMMDevice->Activate(__uuidof(IAudioClient), CLSCTX_ALL, NULL, (void**)&pAudioClient);
		if (FAILED(hr)) 
		{
			Trace(L"IMMDevice::Activate(IAudioClient) failed, hr:[0x%x]", hr);
			return false;
		}
	
		hr = pAudioClient->GetMixFormat(&pWaveFormatEx);
		if (FAILED(hr)) 
		{
			Trace(L"IAudioClient::GetMixFormat failed, hr:[0x%x]", hr);
			return false;
		}

		hr = pAudioClient->Initialize(AUDCLNT_SHAREMODE_SHARED, 0, 10000000, 0, pWaveFormatEx, 0);
		if (FAILED(hr)) 
		{
			Trace(L"IAudioClient::Initialize failed, hr:[0x%x]", hr);
			return false;
		}

		UINT32  bufferFrameCount;
		hr = pAudioClient->GetBufferSize(&bufferFrameCount);
		if (FAILED(hr)) 
		{
			Trace(L"IAudioClient::GetBufferSize failed, hr:[0x%x]", hr);
			return false;
		}

		hr = pAudioClient->GetService(__uuidof(IAudioRenderClient), (void**)&pAudioRenderClient);
		if (FAILED(hr)) 
		{
			Trace(L"IAudioClient::GetService(IAudioRenderClient) failed, hr:[0x%x]", hr);
			return false;
		}

		BYTE* pData;
		hr = pAudioRenderClient->GetBuffer(bufferFrameCount, &pData);
		if (FAILED(hr)) 
		{
			Trace(L"IAudioRenderClient::GetBuffer failed, hr:[0x%x]", hr);
			return false;
		}

		hr = pAudioRenderClient->ReleaseBuffer(bufferFrameCount, AUDCLNT_BUFFERFLAGS_SILENT);
		if (FAILED(hr)) 
		{
			Trace(L"IAudioRenderClient::GetBuffer failed, hr:[0x%x]", hr);
			return false;
		}

		return true;
	}
	__finally
	{
		if (pAudioRenderClient)
			pAudioRenderClient->Release();
		
		if (pWaveFormatEx)
			CoTaskMemFree(pWaveFormatEx);

		if (pAudioClient)
			pAudioClient->Release();
	}
}

bool win7_vis_taskbar::InitializeAudio()
{
	HRESULT hr = CoInitialize(NULL);
	if (FAILED(hr)) 
	{
		Trace(L"CoInitialize failed, hr:[0x%x]", hr);
		return false;
	}

	m_pMMDevice = GetDefaultDevice();
	if (!m_pMMDevice)
	{
		Trace(L"GetDefaultDevice failed");
		return false;
	}

	if (!StutteringFix(m_pMMDevice))
	{
		Trace(L"Stuttering fix failed");
		return false;
	}

	hr = m_pMMDevice->Activate(__uuidof(IAudioClient), CLSCTX_ALL, NULL, (void**)&m_pAudioClient);
	if (FAILED(hr)) 
	{
		Trace(L"IMMDevice::Activate(IAudioClient) failed, hr:[0x%x]", hr);
		return false;
	}

	hr = m_pAudioClient->GetMixFormat(&m_pWaveFormatEx);
	if (FAILED(hr)) 
	{
		Trace(L"IAudioClient::GetMixFormat failed, hr:[0x%x]", hr);
		return false;
	}

	if (m_pWaveFormatEx->wFormatTag == WAVE_FORMAT_EXTENSIBLE)
	{
		m_pWaveFormmatExtensible = (PWAVEFORMATEXTENSIBLE)m_pWaveFormatEx;

		if (m_pWaveFormmatExtensible->SubFormat != KSDATAFORMAT_SUBTYPE_IEEE_FLOAT)
		{
			OLECHAR guid[40];
			StringFromGUID2(KSDATAFORMAT_SUBTYPE_IEEE_FLOAT, guid, 40);
			Trace(L"Unsupported subformat guid:[%ws]", guid);
			return false;
		}

		m_BufferSize = m_pWaveFormatEx->nBlockAlign * FFT_SIZE;
		m_pBuffer = new BYTE[m_BufferSize];
	}
	else
	{
		Trace(L"Unsupported format tag:[0x%x]", m_pWaveFormatEx->wFormatTag);
		return false;
	}

	hr = m_pAudioClient->Initialize(AUDCLNT_SHAREMODE_SHARED, AUDCLNT_STREAMFLAGS_LOOPBACK, 0, 0, m_pWaveFormatEx, 0);
	if (FAILED(hr)) 
	{
		Trace(L"IAudioClient::Initialize failed, hr:[0x%x]", hr);
		return false;
	}

	hr = m_pAudioClient->GetService(__uuidof(IAudioCaptureClient), (void**)&m_pAudioCaptureClient);
	if (FAILED(hr)) 
	{
		Trace(L"IAudioClient::GetService(IAudioCaptureClient) failed, hr:[0x%x]", hr);
		return false;
	}

	hr = m_pAudioClient->Start();
	if (FAILED(hr))
	{
		Trace(L"IAudioClient::Start failed, hr:[0x%x]", hr);
		return false;
	}

	m_hAudioCaptureThread = CreateThread(NULL, 100, (LPTHREAD_START_ROUTINE)AudioCaptureThreadWrapper, this, 0, NULL);
	if (!m_hAudioCaptureThread)
	{
		TraceErr(L"Unable to create rendering thread");
		return false;
	}

	return true;
}

void win7_vis_taskbar::UninitializeAudio()
{
	SetEvent(m_hAudioEvent);

	if (m_hAudioCaptureThread)
	{
		if (WaitForSingleObject(m_hAudioCaptureThread, 5000) != WAIT_OBJECT_0)
		{
			Trace(L"Thread wait timeout occured, trying to terminate");
			if (!TerminateThread(m_hAudioCaptureThread, EXIT_FAILURE))
				TraceErr(L"Unable to terminate thread");
		}

		CloseHandle(m_hAudioCaptureThread);
		m_hAudioCaptureThread = NULL;
	}

	if (m_pAudioCaptureClient)
	{
		m_pAudioCaptureClient->Release();
		m_pAudioCaptureClient = NULL;
	}

	if (m_pWaveFormatEx)
	{
		CoTaskMemFree(m_pWaveFormatEx);
		m_pWaveFormatEx = NULL;
		m_pWaveFormmatExtensible = NULL;
	}

	if (m_pAudioClient)
	{
		m_pAudioClient->Release();
		m_pAudioClient = NULL;
	}

	if (m_pMMDevice)
	{
		m_pMMDevice->Release();
		m_pMMDevice = NULL;
	}

	if (m_pBuffer)
	{
		delete [] m_pBuffer;
		m_pBuffer = NULL;
	}

	CoUninitialize();
}

void win7_vis_taskbar::CloneSettings()
{	
	m_SettingsClone = GetSettings();
}

void win7_vis_taskbar::UpdateSettings()
{
	SetSettings(m_SettingsClone);
}

bool win7_vis_taskbar::FillSpectrumData(PUCHAR values)
{
	__try
	{
		return m_DataIsAvailable;
	}
	__finally
	{
		m_DataIsAvailable = false;
	}
}

DWORD win7_vis_taskbar::AudioCaptureThreadWrapper(LPVOID lpThreadParameter)
{
	return ((win7_vis_taskbar*)lpThreadParameter)->AudioCaptureThread();
}

DWORD win7_vis_taskbar::AudioCaptureThread()
{
	while(IsStarted())
	{
		BYTE *pData;
		UINT32 numFramesToRead;
		DWORD flags;
		HRESULT hr = m_pAudioCaptureClient->GetBuffer(&pData, &numFramesToRead, &flags, NULL, NULL);
		if (FAILED(hr))
		{
			Trace(L"IAudioCaptureClient::GetBuffer failed, hr:[0x%x]", hr);
			return EXIT_FAILURE;
		}

		CopyMemory(m_pBuffer + m_BufferPosition, pData, min(numFramesToRead, m_BufferSize - m_BufferPosition));
		m_BufferPosition += numFramesToRead;

		hr = m_pAudioCaptureClient->ReleaseBuffer(numFramesToRead);
		if (FAILED(hr))
			Trace(L"IAudioCaptureClient::ReleaseBuffer failed, hr:[0x%x]", hr);
	
		if (m_BufferPosition < m_BufferSize)
			continue;
		
		m_pFFTSpectrum->SetSignal((float*)m_pBuffer, FFT_SIGNAL_LEFT);
		float* out = m_pFFTSpectrum->GetReal();
		PUCHAR buffer = GetValuesBuffer();
		int binSize = m_pFFTSpectrum->GetBinSize();

		for(int i=0; i<binSize; i++)
			buffer[i] = (UCHAR)(out[i] * 256);

		m_pFFTSpectrum->SetSignal((float*)m_pBuffer, FFT_SIGNAL_RIGHT);
		out = m_pFFTSpectrum->GetReal();

		for(int i=0; i<binSize; i++)
			buffer[i + binSize] = (UCHAR)(out[i] * 256);
			
		m_BufferPosition = 0;
		ResetEvent(m_hAudioEvent);
		m_DataIsAvailable = true;
		WaitForSingleObject(m_hAudioEvent, INFINITE);
	}

	return EXIT_SUCCESS;
}

void win7_vis_taskbar::MessageLoop()
{
	MSG msg;
	while (GetMessage(&msg, NULL, 0, 0))
	{
		TranslateMessage(&msg);
		DispatchMessage(&msg);
	}
}

void win7_vis_taskbar::ShowContextMenu(HWND hWnd, win7_vis_taskbar* wvt)
{
	POINT pt;
	GetCursorPos(&pt);
	HMENU hMenu = CreatePopupMenu();
	if(!hMenu)
	{
		Trace(L"CreatePopupMenu failed, hr:[0x%x]", HRESULT_FROM_WIN32(GetLastError()));
		return;
	}

	if(IsWindowVisible(hWnd))
		InsertMenu(hMenu, -1, MF_BYPOSITION, ICON_HIDE, L"&Hide Configuration");
	else
		InsertMenu(hMenu, -1, MF_BYPOSITION, ICON_SHOW, L"&Show Configuration");

	if(wvt->IsStarted())
		InsertMenu(hMenu, -1, MF_BYPOSITION, ICON_STOP, L"S&top");
	else
		InsertMenu(hMenu, -1, MF_BYPOSITION, ICON_START, L"S&tarted");

	InsertMenu(hMenu, -1, MF_BYPOSITION, ICON_EXIT, L"&Exit");

	SetForegroundWindow(hWnd);

	TrackPopupMenu(hMenu, TPM_BOTTOMALIGN, pt.x, pt.y, 0, hWnd, NULL );
	DestroyMenu(hMenu);
}

INT_PTR CALLBACK win7_vis_taskbar::PreferencesPageWindowProcedure(HWND hWnd, UINT Msg, WPARAM wParam, LPARAM lParam)
{
	win7_vis_taskbar* wvt = (win7_vis_taskbar*)GetWindowLongPtr(hWnd, GWLP_USERDATA);

    switch(Msg)
    {
        case WM_INITDIALOG:
			{
				TCHAR tmp[6];
				wvt = (win7_vis_taskbar*)lParam;
				SetWindowLongPtr(hWnd, GWLP_USERDATA, (LONG)wvt);

				wvt->LoadConfiguration();
				wvt->CloneSettings();

				_ltot_s(wvt->m_SettingsClone.SleepTime, tmp, 6, 10);
				SetDlgItemText(hWnd, IDC_SLEEPTIME, tmp);

				_ltot_s(wvt->m_SettingsClone.StepMultiplier, tmp, 6, 10);
				SetDlgItemText(hWnd, IDC_STEP, tmp);

				CheckDlgButton(hWnd, IDC_FULLTASKBAR, wvt->m_SettingsClone.FullTaskbar);
				CheckDlgButton(hWnd, IDC_BARS, wvt->m_SettingsClone.Bars);

				SendMessage(GetDlgItem(hWnd, IDC_SLEEPTIMESPIN), UDM_SETBUDDY, (WPARAM)GetDlgItem(hWnd, IDC_SLEEPTIME), 0);
				SendMessage(GetDlgItem(hWnd, IDC_SLEEPTIMESPIN), UDM_SETRANGE, 0, MAKELPARAM(1,1000));
				SendMessage(GetDlgItem(hWnd, IDC_STEPSPIN), UDM_SETBUDDY, (WPARAM)GetDlgItem(hWnd, IDC_STEP), 0);
				SendMessage(GetDlgItem(hWnd, IDC_STEPSPIN), UDM_SETRANGE, 0, MAKELPARAM(1,10));
			}
			break;

		case ICON_MSG:
			switch(lParam)
			{
			case WM_LBUTTONDBLCLK:
				ShowWindow(hWnd, SW_RESTORE);
				break;
			case WM_RBUTTONDOWN:
			case WM_CONTEXTMENU:
				ShowContextMenu(hWnd, wvt);
				break;
			}
			break;

		case WM_CTLCOLORSTATIC:
			{
				LONG id = GetWindowLong((HWND)lParam, GWL_ID);

				switch(id)
				{
				case IDC_TOPCOLOR:
					{
						VISRGB rgb = wvt->m_SettingsClone.RGBTop;
						COLORREF cr = RGB(rgb.R * 255.0f, rgb.G * 255.0f, rgb.B * 255.0f);
						SetBkColor((HDC)wParam, cr);
						return (INT_PTR)CreateSolidBrush(cr);
					}
					break;
				case IDC_BOTTOMCOLOR:
					{
						VISRGB rgb = wvt->m_SettingsClone.RGBBottom;
						COLORREF cr = RGB(rgb.R * 255.0f, rgb.G * 255.0f, rgb.B * 255.0f);
						SetBkColor((HDC)wParam, cr);
						return (INT_PTR)CreateSolidBrush(cr);
					}
					break;
				case IDC_PEAKCOLOR:
					{
						VISRGB rgb = wvt->m_SettingsClone.RGBPeaks;
						COLORREF cr = RGB(rgb.R * 255.0f, rgb.G * 255.0f, rgb.B * 255.0f);
						SetBkColor((HDC)wParam, cr);
						return (INT_PTR)CreateSolidBrush(cr);
					}
					break;
				}
			}
			break;

		case WM_VSCROLL:
			{
				LONG id = GetWindowLong((HWND)lParam, GWL_ID);
				
				switch(id)
				{
				case IDC_SLEEPTIMESPIN:
					{
						TCHAR tmp[6];
						if (!GetDlgItemText(hWnd, IDC_SLEEPTIME, tmp, 6))
						{
							TraceErr(L"GetDlgItemText failed");
							break;
						}

						wvt->m_SettingsClone.SleepTime = _tstol(tmp);
						break;
					}
				case IDC_STEPSPIN:
					{
						TCHAR tmp[6];
						if (!GetDlgItemText(hWnd, IDC_STEP, tmp, 6))
						{
							TraceErr(L"GetDlgItemText failed");
							break;
						}

						wvt->m_SettingsClone.StepMultiplier = _tstol(tmp);
						break;
					}
				}
				break;
			}

        case WM_COMMAND:
            switch (wParam)
			{
			case IDC_FULLTASKBAR:
				wvt->m_SettingsClone.FullTaskbar = IsDlgButtonChecked(hWnd, IDC_FULLTASKBAR) == BST_CHECKED;
				break;
			case IDC_BARS:
				wvt->m_SettingsClone.Bars = IsDlgButtonChecked(hWnd, IDC_BARS) == BST_CHECKED;
				break;
			case IDC_TOPCOLORPICK:
				wvt->m_SettingsClone.RGBTop = ShowColorDialog(hWnd, wvt->m_SettingsClone.RGBTop);
				InvalidateRect(hWnd, NULL, TRUE);
				UpdateWindow(hWnd);
				break;
			case IDC_BOTTOMCOLORPICK:
				wvt->m_SettingsClone.RGBBottom = ShowColorDialog(hWnd, wvt->m_SettingsClone.RGBBottom);
				InvalidateRect(hWnd, NULL, TRUE);
				UpdateWindow(hWnd);
				break;
			case IDC_PEAKCOLORPICK:
				wvt->m_SettingsClone.RGBPeaks = ShowColorDialog(hWnd, wvt->m_SettingsClone.RGBPeaks);
				InvalidateRect(hWnd, NULL, TRUE);
				UpdateWindow(hWnd);
				break;
			case IDC_RESET:
				wvt->CloneSettings();
				break;
			case IDC_APPLY:
				wvt->UpdateSettings();
				wvt->SaveConfiguration();
				break;
			case ICON_HIDE:
			case IDC_CLOSE:
				ShowWindow(hWnd, SW_HIDE);
				break;
			case ICON_SHOW:
				ShowWindow(hWnd, SW_SHOW);
				break;
			case ICON_START:
				if (!wvt->Initialize())
					wvt->Uninitialize();
				break;
			case ICON_STOP:
				wvt->Uninitialize();
				break;
			case ICON_EXIT:
				PostQuitMessage(EXIT_SUCCESS);
				break;
			}
		    break;

		case WM_CLOSE:
			ShowWindow(hWnd, HIDE_WINDOW);
			break;
        
        default: 
            return FALSE;
    }

    return FALSE;
}

VISRGB win7_vis_taskbar::ShowColorDialog(HWND hOwner, VISRGB rgb)
{
	CHOOSECOLOR cc;
	static COLORREF acrCustClr[16];

	ZeroMemory(&cc, sizeof(CHOOSECOLOR));
	cc.lStructSize = sizeof(CHOOSECOLOR);
	cc.lpCustColors = (LPDWORD) acrCustClr;
	cc.hwndOwner = hOwner;
	cc.rgbResult = RGB(rgb.R * 255.0f, rgb.G * 255.0f, rgb.B * 255.0f);
	cc.Flags = CC_FULLOPEN | CC_RGBINIT;

	if (ChooseColor(&cc))
	{
		rgb.R = (FLOAT)GetRValue(cc.rgbResult) / 255.0f;
		rgb.G = (FLOAT)GetGValue(cc.rgbResult) / 255.0f;
		rgb.B = (FLOAT)GetBValue(cc.rgbResult) / 255.0f;
	}

	return rgb;
}


// audio

IMMDevice* win7_vis_taskbar::GetDefaultDevice()
{
	HRESULT hr = S_OK;
	IMMDeviceEnumerator* pMMDeviceEnumerator = NULL;
	IMMDevice* pMMDevice = NULL;

	__try
	{
		hr = CoCreateInstance(__uuidof(MMDeviceEnumerator), NULL, CLSCTX_ALL, __uuidof(IMMDeviceEnumerator), (void**)&pMMDeviceEnumerator);
		if (FAILED(hr)) 
		{
			Trace(L"CoCreateInstance(IMMDeviceEnumerator) failed, hr:[0x%x]", hr);
			return NULL;
		}

		hr = pMMDeviceEnumerator->GetDefaultAudioEndpoint(eRender, eConsole, &pMMDevice);
		if (FAILED(hr)) 
		{
			Trace(L"IMMDeviceEnumerator::GetDefaultAudioEndpoint failed, hr:[0x%x]", hr);
			return NULL;
		}
	}
	__finally
	{
		if (pMMDeviceEnumerator)
			pMMDeviceEnumerator->Release();
	}

    return pMMDevice;
}

bool win7_vis_taskbar::ListDevices()
{
	HRESULT hr = S_OK;
	IMMDeviceEnumerator* pMMDeviceEnumerator = NULL;
	IMMDeviceCollection* pMMDeviceCollection = NULL;

	__try
	{
		hr = CoCreateInstance(__uuidof(MMDeviceEnumerator), NULL, CLSCTX_ALL, __uuidof(IMMDeviceEnumerator), (void**)&pMMDeviceEnumerator);
		if (FAILED(hr)) 
		{
			Trace(L"CoCreateInstance(IMMDeviceEnumerator) failed, hr:[0x%x]", hr);
			return false;
		}

		hr = pMMDeviceEnumerator->EnumAudioEndpoints(eRender, DEVICE_STATE_ACTIVE, &pMMDeviceCollection);
		if (FAILED(hr)) 
		{
			Trace(L"IMMDeviceCollection::EnumAudioEndpoints failed, hr:[0x%x]", hr);
			return false;
		}

		UINT count;
		hr = pMMDeviceCollection->GetCount(&count);
		if (FAILED(hr)) 
		{
			Trace(L"IMMDeviceCollection::GetCount failed, hr:[0x%x]", hr);
			return false;
		}

		for (UINT i = 0; i < count; i++) 
		{
			IMMDevice* pMMDevice = NULL;
			IPropertyStore* pPropertyStore = NULL;
			PROPVARIANT pv;
			PropVariantInit(&pv);

			__try
			{
				hr = pMMDeviceCollection->Item(i, &pMMDevice);
				if (FAILED(hr)) 
				{
					Trace(L"IMMDeviceCollection::Item failed, hr:[0x%x]", hr);
					return false;
				}

				hr = pMMDevice->OpenPropertyStore(STGM_READ, &pPropertyStore);
				if (FAILED(hr)) 
				{
					Trace(L"IMMDevice::OpenPropertyStore failed, hr:[0x%x]", hr);
					return false;
				}
				
				hr = pPropertyStore->GetValue(PKEY_Device_FriendlyName, &pv);
				if (FAILED(hr)) 
				{
					Trace(L"IPropertyStore::GetValue failed, hr:[0x%x]", hr);
					return false;
				}

				if (VT_LPWSTR != pv.vt) 
				{
					Trace(L"Wrong variant type for PKEY_Device_FriendlyName:[%u], expected VT_LPWSTR", pv.vt);
					continue;
				}

				Trace(L"Found Device: [%ls]", pv.pwszVal);
			}
			__finally
			{
				if (pMMDevice)
					pMMDevice->Release();
					
				pMMDevice = NULL;

				if (pPropertyStore)
					pPropertyStore->Release();

				pPropertyStore = NULL;

				PropVariantClear(&pv);
			}
		}    
	}
	__finally
	{
		if (pMMDeviceCollection)
			pMMDeviceCollection->Release();

		if (pMMDeviceEnumerator)
			pMMDeviceEnumerator->Release();
	}

	return true;
}