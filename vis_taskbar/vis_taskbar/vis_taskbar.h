#ifndef VIS_TASKBAR_H
#define VIS_TASKBAR_H

#include "..\..\vis_taskbar_common\vis_taskbar_common.h"

#define DATA_SIZE 1152
#define HALF_DATA_SIZE 576

#define APPNAME L"vis_taskbar"

typedef struct winampVisModule {
  char *description; // description of module
  HWND hwndParent;   // parent window (filled in by calling app)
  HINSTANCE hDllInstance; // instance handle to this DLL (filled in by calling app)
  int sRate;         // sample rate (filled in by calling app)
  int nCh;             // number of channels (filled in...)
  int latencyMs;     // latency from call of RenderFrame to actual drawing
                     // (calling app looks at this value when getting data)
  int delayMs;       // delay between calls in ms

  // the data is filled in according to the respective Nch entry
  int spectrumNch;
  int waveformNch;
  BYTE spectrumData[2][576];
  BYTE waveformData[2][576];

  void (*Config)(struct winampVisModule *this_mod);  // configuration dialog
  int (*Init)(struct winampVisModule *this_mod);     // 0 on success, creates window, etc
  int (*Render)(struct winampVisModule *this_mod);   // returns 0 if successful, 1 if vis should end
  void (*Quit)(struct winampVisModule *this_mod);    // call when done

  void *userData; // user data, optional
} winampVisModule;

typedef struct {
  int version;       // VID_HDRVER
  char *description; // description of library
  winampVisModule* (*getModule)(int);
} winampVisHeader;

// exported symbols
typedef winampVisHeader* (*winampVisGetHeaderType)();

// version of current module (0x100 == 1.00)
#define VIS_HDRVER 0x100
#define MODULEDESC "vis_taskbar (c) vDk. 2010."
#define DLLDESC "vis_taskbar (c) vDk. 2010."

class vis_taskbar : public vis_taskbar_common
{
private:
	static PUCHAR							NewValues;
	static bool								ValuesReady;

	static INT_PTR CALLBACK					PreferencesPageWindowProcedure(HWND hWnd, UINT Msg, WPARAM wParam, LPARAM lParam);
	static VISRGB							ShowColorDialog(HWND hOwner, VISRGB rgb);
	static SETTINGS							SettingsClone;
	
	bool									FillSpectrumData(PUCHAR values);

public:
	static vis_taskbar						Instance;

	void									CloneSettings();
	void									UpdateSettings();	

	static void								ShowConfiguration(struct winampVisModule *this_mod);
	static int								Init(struct winampVisModule *this_mod);
	static int								NewSpectrumData(struct winampVisModule *this_mod);
	static void								Quit(struct winampVisModule *this_mod);
};

#endif //VIS_TASKBAR_H