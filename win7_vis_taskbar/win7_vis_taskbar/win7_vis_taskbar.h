#pragma once

#include <windows.h>
#include <mmsystem.h>
#include <mmdeviceapi.h>
#include <functiondiscoverykeys_devpkey.h>
#include <audioclient.h>
#include <avrt.h>
#include "..\..\vis_taskbar_common\vis_taskbar_common.h"
#include "..\..\fft_spectrum\fft_spectrum.h"

#define APPNAME								L"win7_vis_taskbar"

#define FFT_SIZE							256
#define ICON_UID							459
#define ICON_MSG							WM_APP
#define ICON_SHOW							WM_APP + 1
#define ICON_HIDE							WM_APP + 2
#define ICON_START							WM_APP + 3
#define ICON_STOP							WM_APP + 4
#define ICON_EXIT							WM_APP + 5

class win7_vis_taskbar : protected vis_taskbar_common
{
public:
	win7_vis_taskbar();
	~win7_vis_taskbar();

private:
	SETTINGS								m_SettingsClone;
	UINT									m_BufferPosition;
	UINT									m_BufferSize;

	IMMDevice*								m_pMMDevice;
	IAudioClient*							m_pAudioClient;
	PWAVEFORMATEX							m_pWaveFormatEx;
	PWAVEFORMATEXTENSIBLE					m_pWaveFormmatExtensible;
	IAudioCaptureClient*					m_pAudioCaptureClient;
	PBYTE									m_pBuffer;
	HANDLE									m_hAudioEvent;
	HANDLE									m_hAudioCaptureThread;
	bool									m_DataIsAvailable;
	NOTIFYICONDATA							m_NotifyIcon;
	HWND									m_hPreferencesDialog;

	fft_spectrum*							m_pFFTSpectrum;

	static INT_PTR CALLBACK					PreferencesPageWindowProcedure(HWND hWnd, UINT Msg, WPARAM wParam, LPARAM lParam);
	static VISRGB							ShowColorDialog(HWND hOwner, VISRGB rgb);
	static void								ShowContextMenu(HWND hWnd, win7_vis_taskbar* wvt);

	IMMDevice*								GetDefaultDevice();
	bool									ListDevices();
	bool									FillSpectrumData(PUCHAR values);

	bool									InitializeAudio();
	void									UninitializeAudio();
	static bool								StutteringFix(IMMDevice* pMMDevice);
	DWORD									AudioCaptureThread();
	static DWORD							AudioCaptureThreadWrapper(LPVOID lpThreadParameter);

public:
	HWND									CreatePreferencesDialog(HWND hParent);

	void									CloneSettings();
	void									UpdateSettings();

	bool									BuildUI(HINSTANCE hInstance);
	bool									Initialize();
	void									Uninitialize();

	void									MessageLoop();
};