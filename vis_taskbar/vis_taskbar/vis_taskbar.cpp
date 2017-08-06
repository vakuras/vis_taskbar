#include "vis_taskbar.h"
#include "resource.h"
#include <CommCtrl.h>

EXTERN_C IMAGE_DOS_HEADER __ImageBase;

winampVisModule mod1 =
{
    MODULEDESC,
    NULL,    // hwndParent
    NULL,    // hDllInstance
    0,        // sRate
    0,        // nCh
    0,        // latencyMS - tells winamp how much in advance you want the audio data, 
            //             in ms.
    33,        // delayMS - if winamp tells the plugin to render a frame and it takes
            //           less than this # of milliseconds, winamp will sleep (go idle)
            //           for the remainder.  In effect, this limits the framerate of
            //           the plugin.  A value of 10 would cause a fps limit of ~100.
            //           Derivation: (1000 ms/sec) / (10 ms/frame) = 100 fps.
    2,        // spectrumNch
    0,        // waveformNch
    { 0, },    // spectrumData
    { 0, },    // waveformData
	vis_taskbar::ShowConfiguration,
	vis_taskbar::Init,
    vis_taskbar::NewSpectrumData, 
	vis_taskbar::Quit
};

// getmodule routine from the main header. Returns NULL if an invalid module was requested,
// otherwise returns either mod1, mod2 or mod3 depending on 'which'.
winampVisModule *getModule(int which)
{
    switch (which)
    {
        case 0: return &mod1;
        default: return NULL;
    }
}

// Module header, includes version, description, and address of the module retriever function
winampVisHeader hdr = { VIS_HDRVER, DLLDESC, getModule };

// this is the only exported symbol. returns our main header.
// if you are compiling C++, the extern "C" { is necessary, so we just #ifdef it
#ifdef __cplusplus
extern "C" {
#endif
__declspec( dllexport ) winampVisHeader *winampVisGetHeader()
{
    return &hdr;
}
#ifdef __cplusplus
}
#endif

SETTINGS vis_taskbar::SettingsClone = {0};
PUCHAR vis_taskbar::NewValues = NULL;
bool vis_taskbar::ValuesReady = false;
vis_taskbar vis_taskbar::Instance;

// members

void vis_taskbar::CloneSettings()
{
	SettingsClone = GetSettings();
}

void vis_taskbar::UpdateSettings()
{
	SetSettings(SettingsClone);
}

// static

bool vis_taskbar::FillSpectrumData(PUCHAR values)
{
	NewValues = values;

	if (!ValuesReady)
		return false;

	ValuesReady = false;

	return true;
}

void vis_taskbar::ShowConfiguration(struct winampVisModule *this_mod)
{
	HWND hWnd = NULL;

	__try
	{
		Trace(L"Begin");
		vis_taskbar::Instance.LoadConfiguration();
		vis_taskbar::Instance.CloneSettings();
		DialogBox((HINSTANCE)&__ImageBase, MAKEINTRESOURCE(IDD_TBPREFS), this_mod->hwndParent, (DLGPROC)PreferencesPageWindowProcedure);
	}
	__finally
	{
		Trace(L"End");
	}
}

int vis_taskbar::Init(struct winampVisModule *this_mod)
{
	NewValues = NULL;
	ValuesReady = false;

	if (!vis_taskbar::Instance.IsMeetRequirement())
	{
		Trace(L"OS doesn't meet requirements");
		MessageBox(this_mod->hwndParent, L"You OS doesn't meet the requirements needed for the plugin to work!", L"vis_taskbar", MB_OK | MB_ICONERROR);
		return EXIT_FAILURE;
	}

	if (!vis_taskbar::Instance.StartImpl(DATA_SIZE, NULL))
	{
		Trace(L"Unable to start vis_taskbar");
		return EXIT_FAILURE;
	}

	return EXIT_SUCCESS;
}

int vis_taskbar::NewSpectrumData(struct winampVisModule *this_mod)
{
	if (!NewValues)
		return EXIT_SUCCESS;

	for(int i=0; i<DATA_SIZE; i++)
	{
		if (i >= HALF_DATA_SIZE)
			NewValues[i] = this_mod->spectrumData[1][i - HALF_DATA_SIZE]; //right
		else
			NewValues[i] = this_mod->spectrumData[0][i]; //left
	}

	ValuesReady = true;

	return EXIT_SUCCESS;
}

void vis_taskbar::Quit(struct winampVisModule *this_mod)
{
	vis_taskbar::Instance.StopImpl();
}

INT_PTR CALLBACK vis_taskbar::PreferencesPageWindowProcedure(HWND hWnd, UINT Msg, WPARAM wParam, LPARAM lParam)
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
			case IDC_RESET:
				vis_taskbar::Instance.CloneSettings();
				break;
			case IDC_APPLY:
				vis_taskbar::Instance.UpdateSettings();
				vis_taskbar::Instance.SaveConfiguration();
				break;
			case IDC_CLOSE:
				EndDialog(hWnd, NULL);
				break;
			}
		    break;
        
        default: 
            return FALSE;
    }

    return FALSE;
}

VISRGB vis_taskbar::ShowColorDialog(HWND hOwner, VISRGB rgb)
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