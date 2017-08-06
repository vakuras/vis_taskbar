#pragma once

#include "fft_spectrum.h"
#include "kiss_fftr.h"

class kiss_fft_spectrum : public fft_spectrum 
{
public:
	void									Setup(int signalSize, FFTWindowType windowType);

	~kiss_fft_spectrum();

protected:
	void									ExecuteFFT();
	void									ExecuteIFFT();

private:
	kiss_fftr_cfg							m_FFTCfg;
	kiss_fftr_cfg							m_IFFTCfg;
	float*									m_pWindowedSignal;
	kiss_fft_cpx*							m_pOut;
	kiss_fft_cpx*							m_pIn;
};
