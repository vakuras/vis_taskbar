#include "kiss_fft_spectrum.h"

void kiss_fft_spectrum::Setup(int signalSize, FFTWindowType windowType)
{
	fft_spectrum::Setup(signalSize, windowType);
	m_FFTCfg = kiss_fftr_alloc(signalSize, 0, NULL, NULL);
	m_IFFTCfg = kiss_fftr_alloc(signalSize, 1, NULL, NULL);
	m_pWindowedSignal = new float[signalSize];
	m_pOut = new kiss_fft_cpx[m_BinSize];
	m_pIn = new kiss_fft_cpx[m_BinSize];
}

kiss_fft_spectrum::~kiss_fft_spectrum() 
{
	kiss_fftr_free(m_FFTCfg);
	kiss_fftr_free(m_IFFTCfg);
	delete [] m_pWindowedSignal;
	delete [] m_pOut;
	delete [] m_pIn;
}

void kiss_fft_spectrum::ExecuteFFT() 
{
	memcpy(m_pWindowedSignal, m_pSignal, sizeof(float) * m_SignalSize);
	RunWindow(m_pWindowedSignal);

	kiss_fftr(m_FFTCfg, m_pWindowedSignal, m_pOut);

	for(int i = 0; i < m_BinSize; i++)
	{
		m_pReal[i] = m_pOut[i].r;
		m_pImaginary[i] = m_pOut[i].i;
	}

	m_CartesianUpdated = true;
}

void kiss_fft_spectrum::ExecuteIFFT() 
{
	for(int i = 0; i < m_BinSize; i++) 
	{
		m_pIn[i].r = m_pReal[i];
		m_pIn[i].i = m_pImaginary[i];
	}

	kiss_fftri(m_IFFTCfg, m_pIn, m_pSignal);
	RunInverseWindow(m_pSignal);
	m_SignalUpdated = true;
}
