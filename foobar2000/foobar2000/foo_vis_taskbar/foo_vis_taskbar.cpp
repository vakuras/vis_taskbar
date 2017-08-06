#include "foo_vis_taskbar.h"
#include "resource.h"
#include <Commctrl.h>

EXTERN_C IMAGE_DOS_HEADER __ImageBase;

SETTINGS foo_vis_taskbar::SettingsClone = {0};

foo_vis_taskbar foo_vis_taskbar::Instance;

// members

void foo_vis_taskbar::Start()
{
	if (!foo_vis_taskbar::IsMeetRequirement())
	{
		Trace(L"OS doesn't meet requirements");
		MessageBox(NULL, L"You OS doesn't meet the requirements needed for the plugin to work!", L"vis_taskbar", MB_OK | MB_ICONERROR);
		return;
	}

	if (!CreateStream())
		return;
	
	StartImpl(DATA_SIZE, NULL);
}

void foo_vis_taskbar::Stop()
{
	StopImpl();
}

void foo_vis_taskbar::CloneSettings()
{
	SettingsClone = GetSettings();
}

void foo_vis_taskbar::UpdateSettings()
{
	SetSettings(SettingsClone);
	SaveConfiguration();
}

HWND foo_vis_taskbar::CreatePreferencesDialog(HWND hParent)
{
	__try
	{
		Trace(L"Begin");

		
		HWND hWnd = CreateDialog((HINSTANCE)&__ImageBase, MAKEINTRESOURCE(IDD_TBPREFS), hParent, (DLGPROC)PreferencesPageWindowProcedure);

		if (!hWnd)
			TraceErr(L"CreateDialog failed");

		return hWnd;
	}
	__finally
	{
		Trace(L"End");
	}
}

// static

service_ptr_t<visualisation_stream> foo_vis_taskbar::Stream = NULL;

bool foo_vis_taskbar::CreateStream()
{
	if (Stream != NULL)
		return true;

	try 
	{
		static_api_ptr_t<visualisation_manager>()->create_stream(Stream, visualisation_manager::KStreamFlagNewFFT);
		return true;
	} 
	catch (const std::exception & exc) 
	{
		Trace(L"Exception while creating visualisation stream: %s", exc.what());
		popup_message::g_show(pfc::string8() << "Exception while creating visualisation stream:\n" << exc, "Error", popup_message::icon_error);
		return false;
	}
}

INT_PTR CALLBACK foo_vis_taskbar::PreferencesPageWindowProcedure(HWND hWnd, UINT Msg, WPARAM wParam, LPARAM lParam)
{
    switch(Msg)
    {
        case WM_INITDIALOG:
			{
				TCHAR tmp[6];

				_ltot_s(SettingsClone.SleepTime, tmp, 6, 10);
				SetDlgItemText(hWnd, IDC_SLEEPTIME, tmp);

				_ltot_s(SettingsClone.StepMultiplier, tmp, 6, 10);
				SetDlgItemText(hWnd, IDC_STEP, tmp);

				CheckDlgButton(hWnd, IDC_FULLTASKBAR, SettingsClone.FullTaskbar);
				CheckDlgButton(hWnd, IDC_BARS, SettingsClone.Bars);

				SendMessage(GetDlgItem(hWnd, IDC_SLEEPTIMESPIN), UDM_SETBUDDY, (WPARAM)GetDlgItem(hWnd, IDC_SLEEPTIME), 0);
				SendMessage(GetDlgItem(hWnd, IDC_SLEEPTIMESPIN), UDM_SETRANGE, 0, MAKELPARAM(1,1000));
				SendMessage(GetDlgItem(hWnd, IDC_STEPSPIN), UDM_SETBUDDY, (WPARAM)GetDlgItem(hWnd, IDC_STEP), 0);
				SendMessage(GetDlgItem(hWnd, IDC_STEPSPIN), UDM_SETRANGE, 0, MAKELPARAM(1,10));
			}
			break;

		case WM_CTLCOLORSTATIC:
			{
				LONG id = GetWindowLong((HWND)lParam, GWL_ID);

				switch(id)
				{
				case IDC_TOPCOLOR:
					{
						VISRGB rgb = SettingsClone.RGBTop;
						COLORREF cr = RGB(rgb.R * 255.0f, rgb.G * 255.0f, rgb.B * 255.0f);
						SetBkColor((HDC)wParam, cr);
						return (INT_PTR)CreateSolidBrush(cr);
					}
					break;
				case IDC_BOTTOMCOLOR:
					{
						VISRGB rgb = SettingsClone.RGBBottom;
						COLORREF cr = RGB(rgb.R * 255.0f, rgb.G * 255.0f, rgb.B * 255.0f);
						SetBkColor((HDC)wParam, cr);
						return (INT_PTR)CreateSolidBrush(cr);
					}
					break;
				case IDC_PEAKCOLOR:
					{
						VISRGB rgb = SettingsClone.RGBPeaks;
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

						SettingsClone.SleepTime = _tstol(tmp);
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

						SettingsClone.StepMultiplier = _tstol(tmp);
						break;
					}
				}
				break;
			}

        case WM_COMMAND:
            switch (wParam)
			{
			case IDC_FULLTASKBAR:
				SettingsClone.FullTaskbar = IsDlgButtonChecked(hWnd, IDC_FULLTASKBAR) == BST_CHECKED;
				break;
			case IDC_BARS:
				SettingsClone.Bars = IsDlgButtonChecked(hWnd, IDC_BARS) == BST_CHECKED;
				break;
			case IDC_TOPCOLORPICK:
				SettingsClone.RGBTop = ShowColorDialog(hWnd, SettingsClone.RGBTop);
				InvalidateRect(hWnd, NULL, TRUE);
				UpdateWindow(hWnd);
				break;
			case IDC_BOTTOMCOLORPICK:
				SettingsClone.RGBBottom = ShowColorDialog(hWnd, SettingsClone.RGBBottom);
				InvalidateRect(hWnd, NULL, TRUE);
				UpdateWindow(hWnd);
				break;
			case IDC_PEAKCOLORPICK:
				SettingsClone.RGBPeaks = ShowColorDialog(hWnd, SettingsClone.RGBPeaks);
				InvalidateRect(hWnd, NULL, TRUE);
				UpdateWindow(hWnd);
				break;
			case IDC_START:
				foo_vis_taskbar::Instance.Start();
				break;
			case IDC_STOP:
				foo_vis_taskbar::Instance.Stop();
				break;
			}
		    break;

        case WM_DESTROY:
	        break;
        
        default:
            return FALSE;
    }
    return 0;
}

VISRGB foo_vis_taskbar::ShowColorDialog(HWND hOwner, VISRGB rgb)
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

bool foo_vis_taskbar::FillSpectrumData(PUCHAR values)
{
	double absTime;
	audio_chunk_impl chunk;
	audio_sample* data;

	if (!Stream->get_absolute_time(absTime))
		return false;

	if (!Stream->get_chunk_absolute(chunk, absTime, .1f))
		return false;

	if (!Stream->get_spectrum_absolute(chunk, absTime, FFT_SIZE))
		return false;

	data = chunk.get_data();
	for(int i=0; i<DATA_SIZE; i++)
	{
		if (i>HALF_DATA_SIZE_MINUS_ONE)
			values[i] = (UCHAR)(data[(i - HALF_DATA_SIZE) *2 + 1]*32768.0);
		else
			values[i] = (UCHAR)(data[i*2]*32768.0);
	}

	return true;
}
















